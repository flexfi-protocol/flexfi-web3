use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, Sysvar, rent::Rent},
    msg,
};
use borsh::{BorshDeserialize, BorshSerialize};
use spl_associated_token_account;
use crate::error::FlexfiError;
use crate::state::{staking::{StakingAccount, StakingStatus}};
use crate::constants::{STAKING_SEED, USDC_VAULT_SEED, MIN_STAKING_AMOUNT, MIN_STAKING_LOCK_DAYS, MAX_STAKING_LOCK_DAYS};
use crate::core::whitelist::require_whitelisted;

pub fn process_deposit_staking(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    lock_days: u16,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let staking_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?; // Whitelist account
    let user_token_account = next_account_info(account_info_iter)?;
    let vault_token_account = next_account_info(account_info_iter)?;
    let usdc_mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let associated_token_program = next_account_info(account_info_iter)?;
    let _rent_sysvar = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check user signature
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Check if the user is whitelisted
    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;

    // Check minimum amount
    if amount < MIN_STAKING_AMOUNT {
        return Err(FlexfiError::InsufficientStaking.into());
    }

    // Check lock period
    if lock_days < MIN_STAKING_LOCK_DAYS || lock_days > MAX_STAKING_LOCK_DAYS {
        return Err(ProgramError::InvalidArgument);
    }

    // Find the PDA of the staking account
    let seeds = [
        STAKING_SEED,
        user_account.key.as_ref(),
        usdc_mint.key.as_ref(),
    ];
    let (staking_pda, staking_bump) = Pubkey::find_program_address(&seeds, program_id);

    msg!("Calculated staking PDA: {}", staking_pda);
    msg!("Received staking account: {}", staking_account.key);
    msg!("Staking bump: {}", staking_bump);

    if *staking_account.key != staking_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Find the PDA of the vault
    let vault_seeds = [
        USDC_VAULT_SEED,
        staking_account.key.as_ref(),
    ];
    let (vault_pda, vault_bump) = Pubkey::find_program_address(&vault_seeds, program_id);

    msg!("Calculated vault PDA: {}", vault_pda);
    msg!("Received vault account: {}", vault_token_account.key);
    msg!("Vault bump: {}", vault_bump);

    // Get current time
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Initialize or update the staking account
    let mut staking_data = if !staking_account.data_is_empty() {
        // Existing account, load data
        let mut data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;

        // Check that staking is active or locked
        let status = data.get_status()?;
        if status != StakingStatus::Active && status != StakingStatus::Locked {
            return Err(FlexfiError::StakingFrozen.into());
        }

        // Update amounts and lock period
        data.amount_staked = data.amount_staked.saturating_add(amount);

        if status == StakingStatus::Locked {
            let new_lock_end = current_time + (lock_days as i64 * 86400);
            if new_lock_end > data.lock_period_end {
                data.lock_period_end = new_lock_end;
            }
        } else {
            data.set_status(StakingStatus::Locked);
            data.lock_period_end = current_time + (lock_days as i64 * 86400);
        }

        data.last_update = current_time;
        data
    } else {
        // New staking account to create
        let rent = Rent::get()?;
        let space = StakingAccount::SIZE;
        let rent_lamports = rent.minimum_balance(space);

        msg!("Creating staking account with size: {}", space);

        invoke_signed(
            &system_instruction::create_account(
                user_account.key,
                &staking_pda,
                rent_lamports,
                space as u64,
                program_id,
            ),
            &[user_account.clone(), staking_account.clone(), system_program.clone()],
            &[&[STAKING_SEED, user_account.key.as_ref(), usdc_mint.key.as_ref(), &[staking_bump]]],
        )?;

        // Create the vault ATA if necessary
        if vault_token_account.data_is_empty() {
            invoke_signed(
                &spl_associated_token_account::instruction::create_associated_token_account(
                    user_account.key,
                    &vault_pda,
                    usdc_mint.key,
                    &spl_token::id(),
                ),
                &[
                    user_account.clone(),
                    vault_token_account.clone(),
                    usdc_mint.clone(),
                    system_program.clone(),
                    token_program.clone(),
                    associated_token_program.clone(),
                ],
                &[&[USDC_VAULT_SEED, staking_account.key.as_ref(), &[vault_bump]]],
            )?;
        }

        // Initialize staking data
        StakingAccount::new(
            *user_account.key,
            *usdc_mint.key,
            amount,
            StakingStatus::Locked,
            current_time + (lock_days as i64 * 86400),
            current_time,
            staking_bump,
        )
    };

    // Save staking data
    staking_data.serialize(&mut *staking_account.data.borrow_mut())?;

    // Transfer USDC to the vault
    let transfer_ix = spl_token::instruction::transfer(
        token_program.key,
        user_token_account.key,
        vault_token_account.key,
        user_account.key,
        &[],
        amount,
    )?;

    invoke(
        &transfer_ix,
        &[
            user_token_account.clone(),
            vault_token_account.clone(),
            user_account.clone(),
            token_program.clone(),
        ],
    )?;

    msg!("Staking deposit successful: {} units, locked for {} days", amount, lock_days);
    Ok(())
}

pub fn process_withdraw_staking(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let staking_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?; // Whitelist account
    let user_token_account = next_account_info(account_info_iter)?;
    let vault_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check user signature
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // CHECK IF THE USER IS WHITELISTED
    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;

    // Load staking data
    let mut staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;

    // Verify that the user is the owner
    if staking_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Check staking status
    let status = staking_data.get_status()?;
    if status == StakingStatus::Frozen || status == StakingStatus::Closed {
        return Err(FlexfiError::StakingFrozen.into());
    }

    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // If locked, check if the lock period has ended
    if status == StakingStatus::Locked && current_time < staking_data.lock_period_end {
        return Err(FlexfiError::StakingFrozen.into());
    }

    // Check if the requested amount is available
    if amount > staking_data.amount_staked {
        return Err(FlexfiError::InsufficientStaking.into());
    }

    // Update the staked amount
    staking_data.amount_staked = staking_data.amount_staked.saturating_sub(amount);
    staking_data.last_update = current_time;

    // If the remaining amount is less than the minimum, close the account
    if staking_data.amount_staked < MIN_STAKING_AMOUNT {
        staking_data.set_status(StakingStatus::Closed);
    } else {
        // Otherwise, set to active status
        staking_data.set_status(StakingStatus::Active);
    }

    // Save changes
    staking_data.serialize(&mut *staking_account.data.borrow_mut())?;

    // Prepare seeds to sign with the vault PDA
    let vault_seeds = [
        USDC_VAULT_SEED,
        staking_account.key.as_ref(),
        &[staking_data.bump],
    ];

    // Transfer tokens from the vault to the user
    let transfer_ix = spl_token::instruction::transfer(
        token_program.key,
        vault_token_account.key,
        user_token_account.key,
        &staking_account.key, // The staking account is the vault's authority
        &[],
        amount,
    )?;

    invoke_signed(
        &transfer_ix,
        &[
            vault_token_account.clone(),
            user_token_account.clone(),
            staking_account.clone(),
            token_program.clone(),
        ],
        &[&vault_seeds],
    )?;

    msg!("Staking withdrawal successful: {} units", amount);
    Ok(())
}

pub fn process_check_unlock_status(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let staking_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?; // Whitelist account
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check user signature
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // CHECK IF THE USER IS WHITELISTED
    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;

    // Load staking data
    let mut staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;

    // Verify that the user is the owner
    if staking_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Check if staking is locked
    let status = staking_data.get_status()?;
    if status != StakingStatus::Locked {
        msg!("Staking is not locked");
        return Ok(());
    }

    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Check if the lock period has ended
    if current_time >= staking_data.lock_period_end {
        // Unlock staking
        staking_data.set_status(StakingStatus::Active);
        staking_data.last_update = current_time;

        // Save changes
        staking_data.serialize(&mut *staking_account.data.borrow_mut())?;

        msg!("Staking unlocked: lock period has ended");
    } else {
        let remaining_time = staking_data.lock_period_end - current_time;
        let remaining_days = remaining_time / 86400;

        msg!("Staking still locked: {} days remaining", remaining_days);
    }

    Ok(())
}

// Manager for staking functions
pub struct StakingManager;

impl StakingManager {
    pub fn deposit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        lock_days: u16,
    ) -> ProgramResult {
        process_deposit_staking(program_id, accounts, amount, lock_days)
    }

    pub fn withdraw(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        process_withdraw_staking(program_id, accounts, amount)
    }

    pub fn check_unlock(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        process_check_unlock_status(program_id, accounts)
    }
}
