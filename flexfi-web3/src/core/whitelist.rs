use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program::invoke_signed,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, Sysvar, rent::Rent},
    msg,
};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::error::FlexfiError;
use crate::state::whitelist::{WhitelistAccount, UserWhitelistStatus};
use crate::constants::{WHITELIST_SEED};

pub fn check_user_whitelisted(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    accounts: &[AccountInfo],
) -> Result<bool, ProgramError> {
    // Check the on-chain whitelist
    let account_info_iter = &mut accounts.iter();
    let user_status_account = next_account_info(account_info_iter)?;

    // Check the PDA
    let (user_status_pda, _) = Pubkey::find_program_address(
        &[WHITELIST_SEED, user_pubkey.as_ref()],
        program_id
    );

    if user_status_account.key != &user_status_pda {
        return Ok(false);
    }

    // If the account doesn't exist, the user is not whitelisted
    if user_status_account.data_is_empty() {
        return Ok(false);
    }

    // Load and check the status
    let user_status = UserWhitelistStatus::try_from_slice(&user_status_account.data.borrow())?;

    Ok(user_status.is_whitelisted)
}

// Helper function that generates an error if the user is not whitelisted
pub fn require_whitelisted(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    user_status_account: &AccountInfo,
) -> ProgramResult {
    let is_whitelisted = check_user_whitelisted(
        program_id,
        user_pubkey,
        &[user_status_account.clone()]
    )?;

    if !is_whitelisted {
        msg!("User {} is not whitelisted and cannot use this function", user_pubkey);
        return Err(FlexfiError::Unauthorized.into());
    }

    Ok(())
}

// Initialize the whitelist (called once by an admin)
pub fn process_initialize_whitelist(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let whitelist_account = next_account_info(account_info_iter)?;
    let authority = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify that the authority is the signer
    if !authority.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Create the PDA for the whitelist
    let (whitelist_pda, bump) = Pubkey::find_program_address(
        &[WHITELIST_SEED],
        program_id
    );

    if whitelist_account.key != &whitelist_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create the account
    let rent = Rent::get()?;
    let space = WhitelistAccount::SIZE;
    let rent_lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            authority.key,
            &whitelist_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[authority.clone(), whitelist_account.clone(), system_program.clone()],
        &[&[WHITELIST_SEED, &[bump]]],
    )?;

    // Initialize the data
    let whitelist_data = WhitelistAccount {
        authority: *authority.key,
        is_active: true,
        total_users: 0,
        bump,
    };

    whitelist_data.serialize(&mut *whitelist_account.data.borrow_mut())?;

    msg!("Whitelist initialized with authority: {}", authority.key);
    Ok(())
}

// Add a user to the whitelist (called by the backend)
pub fn process_add_to_whitelist(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    user_pubkey: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let whitelist_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let authority = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Verify the authority
    if !authority.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Load the whitelist
    let mut whitelist_data = WhitelistAccount::try_from_slice(&whitelist_account.data.borrow())?;

    // Verify that the authority is correct
    if whitelist_data.authority != *authority.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Create the PDA for the user's status
    let (user_status_pda, user_bump) = Pubkey::find_program_address(
        &[WHITELIST_SEED, user_pubkey.as_ref()],
        program_id
    );

    if user_status_account.key != &user_status_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Get the timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;

    // Create the user status account
    let rent = Rent::get()?;
    let space = UserWhitelistStatus::SIZE;
    let rent_lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            authority.key,
            &user_status_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[authority.clone(), user_status_account.clone(), system_program.clone()],
        &[&[WHITELIST_SEED, user_pubkey.as_ref(), &[user_bump]]],
    )?;

    // Initialize the status
    let user_status = UserWhitelistStatus {
        user_pubkey,
        is_whitelisted: true,
        whitelisted_at: clock.unix_timestamp,
        whitelisted_by: *authority.key,
        bump: user_bump,
    };

    user_status.serialize(&mut *user_status_account.data.borrow_mut())?;

    // Update the counter
    whitelist_data.total_users += 1;
    whitelist_data.serialize(&mut *whitelist_account.data.borrow_mut())?;

    msg!("User {} added to whitelist", user_pubkey);
    Ok(())
}

pub fn process_remove_from_whitelist(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    user_pubkey: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let whitelist_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let authority = next_account_info(account_info_iter)?;

    // Verify the authority
    if !authority.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Load the whitelist
    let mut whitelist_data = WhitelistAccount::try_from_slice(&whitelist_account.data.borrow())?;

    // Verify that the authority is correct
    if whitelist_data.authority != *authority.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Verify the user status PDA
    let (user_status_pda, _) = Pubkey::find_program_address(
        &[WHITELIST_SEED, user_pubkey.as_ref()],
        program_id
    );

    if user_status_account.key != &user_status_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Load the user status
    let mut user_status = UserWhitelistStatus::try_from_slice(&user_status_account.data.borrow())?;

    // Verify that it's the correct user
    if user_status.user_pubkey != user_pubkey {
        return Err(ProgramError::InvalidAccountData);
    }

    // Mark as not whitelisted
    user_status.is_whitelisted = false;
    user_status.serialize(&mut *user_status_account.data.borrow_mut())?;

    // Decrement the counter (beware of underflows)
    whitelist_data.total_users = whitelist_data.total_users.saturating_sub(1);
    whitelist_data.serialize(&mut *whitelist_account.data.borrow_mut())?;

    msg!("User {} removed from whitelist", user_pubkey);
    Ok(())
}
