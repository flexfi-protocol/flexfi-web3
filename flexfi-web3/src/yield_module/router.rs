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
use crate::core::whitelist::require_whitelisted;
use crate::error::FlexfiError;
use crate::state::yield_::{YieldAccount, YieldStrategy};
use crate::constants::{YIELD_CONFIG_SEED};

pub fn process_set_yield_strategy(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    strategy: u8,
    auto_reinvest: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let yield_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Verify user signature
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;

    // Convert u8 to YieldStrategy
    let yield_strategy = YieldStrategy::from_u8(strategy)?;

    // Get custom strategy address if it's a custom strategy
    let custom_strategy_address = if yield_strategy == YieldStrategy::Custom && account_info_iter.len() > 0 {
        let custom_account = next_account_info(account_info_iter)?;
        *custom_account.key
    } else {
        Pubkey::default()
    };

    // Verify that the custom address is valid for a custom strategy
    if yield_strategy == YieldStrategy::Custom && custom_strategy_address == Pubkey::default() {
        return Err(ProgramError::InvalidArgument);
    }

    // Create a PDA for the yield account
    let seeds = [
        YIELD_CONFIG_SEED,
        user_account.key.as_ref(),
    ];
    let (yield_pda, bump_seed) = Pubkey::find_program_address(&seeds, program_id);

    if *yield_account.key != yield_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Create or update the yield account
    if yield_account.owner == program_id {
        // Existing account, update the strategy
        let mut yield_data = YieldAccount::try_from_slice(&yield_account.data.borrow())?;

        // Verify ownership
        if yield_data.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }

        // Update the strategy
        yield_data.set_strategy(yield_strategy);
        yield_data.custom_strategy_address = custom_strategy_address;
        yield_data.auto_reinvest = auto_reinvest;

        // Save changes
        yield_data.serialize(&mut *yield_account.data.borrow_mut())?;
    } else {
        // New account, create it
        let rent = Rent::get()?;
        let space = YieldAccount::SIZE;
        let rent_lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                user_account.key,
                &yield_pda,
                rent_lamports,
                space as u64,
                program_id,
            ),
            &[user_account.clone(), yield_account.clone(), system_program.clone()],
            &[&[YIELD_CONFIG_SEED, user_account.key.as_ref(), &[bump_seed]]],
        )?;

        // Initialize the account
        let yield_data = YieldAccount::new(
            *user_account.key,
            yield_strategy,
            custom_strategy_address,
            auto_reinvest,
            current_time,
            bump_seed,
        );

        // Save data
        yield_data.serialize(&mut *yield_account.data.borrow_mut())?;
    }

    msg!("Yield strategy set to: {:?}, auto-reinvest: {}", yield_strategy, auto_reinvest);
    Ok(())
}

pub fn process_route_yield(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let yield_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let source_token_account = next_account_info(account_info_iter)?;
    let destination_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Verify user signature
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Load yield data
    let mut yield_data = YieldAccount::try_from_slice(&yield_account.data.borrow())?;

    // Verify ownership
    if yield_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Get the strategy
    let strategy = yield_data.get_strategy()?;

    // Route yield based on the strategy
    match strategy {
        YieldStrategy::AutoCompound => {
            // Route to AutoCompound strategy
            msg!("Routing yield to AutoCompound strategy: {}", amount);

            // Transfer to auto-compound strategy
            let transfer_ix = spl_token::instruction::transfer(
                token_program.key,
                source_token_account.key,
                destination_token_account.key,
                user_account.key,
                &[],
                amount,
            )?;

            invoke(
                &transfer_ix,
                &[
                    source_token_account.clone(),
                    destination_token_account.clone(),
                    user_account.clone(),
                    token_program.clone(),
                ],
            )?;
        },
        YieldStrategy::StableCoin => {
            // Convert to stablecoin
            msg!("Routing yield to StableCoin strategy: {}", amount);

            // Similar transfer
            let transfer_ix = spl_token::instruction::transfer(
                token_program.key,
                source_token_account.key,
                destination_token_account.key,
                user_account.key,
                &[],
                amount,
            )?;

            invoke(
                &transfer_ix,
                &[
                    source_token_account.clone(),
                    destination_token_account.clone(),
                    user_account.clone(),
                    token_program.clone(),
                ],
            )?;
        },
        YieldStrategy::HighYield => {
            // Route to high yield strategy
            msg!("Routing yield to HighYield strategy: {}", amount);

            // Transfer to high yield strategy
            let transfer_ix = spl_token::instruction::transfer(
                token_program.key,
                source_token_account.key,
                destination_token_account.key,
                user_account.key,
                &[],
                amount,
            )?;

            invoke(
                &transfer_ix,
                &[
                    source_token_account.clone(),
                    destination_token_account.clone(),
                    user_account.clone(),
                    token_program.clone(),
                ],
            )?;
        },
        YieldStrategy::RealWorldAssets => {
            // Route to real world assets strategy
            msg!("Routing yield to RealWorldAssets strategy: {}", amount);

            // Transfer to real world assets strategy
            let transfer_ix = spl_token::instruction::transfer(
                token_program.key,
                source_token_account.key,
                destination_token_account.key,
                user_account.key,
                &[],
                amount,
            )?;

            invoke(
                &transfer_ix,
                &[
                    source_token_account.clone(),
                    destination_token_account.clone(),
                    user_account.clone(),
                    token_program.clone(),
                ],
            )?;
        },
        YieldStrategy::Custom => {
            // Route to custom strategy
            msg!("Routing yield to Custom strategy at {}: {}",
                 yield_data.custom_strategy_address, amount);

            // Transfer to custom strategy
            let transfer_ix = spl_token::instruction::transfer(
                token_program.key,
                source_token_account.key,
                destination_token_account.key,
                user_account.key,
                &[],
                amount,
            )?;

            invoke(
                &transfer_ix,
                &[
                    source_token_account.clone(),
                    destination_token_account.clone(),
                    user_account.clone(),
                    token_program.clone(),
                ],
            )?;
        },
    }

    // Record yield earned
    yield_data.record_yield_earned(amount);

    // Update last yield date
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    yield_data.last_yield_claimed = current_time;

    // Save changes
    yield_data.serialize(&mut *yield_account.data.borrow_mut())?;

    Ok(())
}

pub struct YieldRouter;

impl YieldRouter {
    pub fn route_yield(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        process_route_yield(program_id, accounts, amount)
    }
}
