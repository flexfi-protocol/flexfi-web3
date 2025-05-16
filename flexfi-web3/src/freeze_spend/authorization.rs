use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, Sysvar, rent::Rent},
    msg,
    program_error::ProgramError,
};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::error::FlexfiError;
use crate::state::authorization::AuthorizationAccount;
use crate::state::staking::StakingAccount;
use crate::constants::{AUTHORIZATION_SEED, FLEXFI_AUTHORITY_SEED, USDC_VAULT_SEED};
use crate::core::whitelist::require_whitelisted;

pub fn process_initialize_flexfi_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    authorized_amount: u64,
    duration_days: u16,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authorization_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let staking_account = next_account_info(account_info_iter)?;
    let flexfi_authority_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check user signature
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Check if the user is whitelisted
    require_whitelisted(program_id, user_account.key, user_status_account)?;

    // Check if the user has sufficient staking
    let staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;
    if staking_data.amount_staked < authorized_amount {
        return Err(FlexfiError::InsufficientStaking.into());
    }

    // Create the PDA for authorization
    let (authorization_pda, auth_bump) = Pubkey::find_program_address(
        &[AUTHORIZATION_SEED, user_account.key.as_ref()],
        program_id
    );

    if *authorization_account.key != authorization_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify the FlexFi authority (program PDA)
    let (flexfi_authority_pda, _) = Pubkey::find_program_address(
        &[FLEXFI_AUTHORITY_SEED],
        program_id
    );

    if *flexfi_authority_account.key != flexfi_authority_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create the authorization account
    let rent = Rent::get()?;
    let space = AuthorizationAccount::SIZE;
    let rent_lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            user_account.key,
            &authorization_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[user_account.clone(), authorization_account.clone(), system_program.clone()],
        &[&[AUTHORIZATION_SEED, user_account.key.as_ref(), &[auth_bump]]],
    )?;

    // Initialize the data
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    let expires_at = current_time + (duration_days as i64 * 86400);

    let authorization = AuthorizationAccount {
        user: *user_account.key,
        flexfi_authority: flexfi_authority_pda,
        authorized_amount,
        used_amount: 0,
        is_active: true,
        created_at: current_time,
        expires_at,
        bump: auth_bump,
    };

    authorization.serialize(&mut *authorization_account.data.borrow_mut())?;

    msg!("FlexFi account initialized: {} USDC authorized for {} days",
         authorized_amount / 1_000_000, duration_days);
    Ok(())
}

pub fn process_flexfi_spend(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    merchant: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authorization_account = next_account_info(account_info_iter)?;
    let user_staking_account = next_account_info(account_info_iter)?;
    let staking_vault_account = next_account_info(account_info_iter)?;
    let merchant_token_account = next_account_info(account_info_iter)?;
    let flexfi_authority_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Load authorization data
    let mut authorization = AuthorizationAccount::try_from_slice(
        &authorization_account.data.borrow()
    )?;

    // Verify the FlexFi authority
    let (flexfi_authority_pda, flexfi_bump) = Pubkey::find_program_address(
        &[FLEXFI_AUTHORITY_SEED],
        program_id
    );

    if *flexfi_authority_account.key != flexfi_authority_pda {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Verify the validity of the authorization
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    if !authorization.is_valid(current_time) {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Check if the credit is sufficient
    if authorization.remaining_credit() < amount {
        return Err(FlexfiError::InsufficientCollateral.into());
    }

    // Perform the transfer from the staking vault
    let staking_data = StakingAccount::try_from_slice(&user_staking_account.data.borrow())?;

    let _staking_seeds = [
        USDC_VAULT_SEED,
        user_staking_account.key.as_ref(),
        &[staking_data.bump],
    ];

    let transfer_ix = spl_token::instruction::transfer(
        token_program.key,
        staking_vault_account.key,
        merchant_token_account.key,
        &flexfi_authority_pda, // FlexFi has the authority!
        &[],
        amount,
    )?;

    invoke_signed(
        &transfer_ix,
        &[
            staking_vault_account.clone(),
            merchant_token_account.clone(),
            flexfi_authority_account.clone(),
            token_program.clone(),
        ],
        &[&[FLEXFI_AUTHORITY_SEED, &[flexfi_bump]]],
    )?;

    // Update the used amount
    authorization.used_amount = authorization.used_amount.saturating_add(amount);
    authorization.serialize(&mut *authorization_account.data.borrow_mut())?;

    msg!("FlexFi spend: {} USDC to merchant {}", amount / 1_000_000, merchant);
    msg!("Remaining credit: {} USDC", authorization.remaining_credit() / 1_000_000);

    Ok(())
}

pub fn process_revoke_authorization(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authorization_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;

    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    let mut authorization = AuthorizationAccount::try_from_slice(
        &authorization_account.data.borrow()
    )?;

    if authorization.user != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    authorization.is_active = false;
    authorization.serialize(&mut *authorization_account.data.borrow_mut())?;

    msg!("Authorization revoked by user");
    Ok(())
}
