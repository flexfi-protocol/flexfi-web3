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
use crate::core::whitelist::require_whitelisted;
use crate::error::FlexfiError;
use crate::state::score::ScoreAccount;
use crate::constants::{SCORE_SEED, INITIAL_SCORE};

pub fn process_initialize_score(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let score_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check user signature
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;

    // Create a PDA for the score account
    let seeds = [
        SCORE_SEED,
        user_account.key.as_ref(),
    ];
    let (score_pda, bump_seed) = Pubkey::find_program_address(&seeds, program_id);

    if *score_account.key != score_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if the account already exists
    if score_account.owner == program_id {
        msg!("Score account already exists");
        return Ok(());
    }

    // Create the score account
    let rent = Rent::get()?;
    let space = ScoreAccount::SIZE;
    let rent_lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            user_account.key,
            &score_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[user_account.clone(), score_account.clone(), system_program.clone()],
        &[&[SCORE_SEED, user_account.key.as_ref(), &[bump_seed]]],
    )?;

    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Initialize the account with an initial score
    let score_data = ScoreAccount::new(
        *user_account.key,
        INITIAL_SCORE,
        current_time,
        bump_seed,
    );

    // Save data
    score_data.serialize(&mut *score_account.data.borrow_mut())?;

    msg!("Score initialized with initial score of {}", INITIAL_SCORE);
    Ok(())
}

pub fn process_update_score(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    change: i16,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let score_account = next_account_info(account_info_iter)?;
    let authority_account = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check authority signature
    // In a real implementation, check if the authority is authorized
    if !authority_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Load score data
    let mut score_data = ScoreAccount::try_from_slice(&score_account.data.borrow())?;

    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Update the score
    score_data.update_score(change, current_time);

    // Save changes
    score_data.serialize(&mut *score_account.data.borrow_mut())?;

    msg!("Score updated: new score = {}", score_data.score);
    Ok(())
}

pub fn process_record_new_loan(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let score_account = next_account_info(account_info_iter)?;
    let authority_account = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check authority signature
    if !authority_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Load score data
    let mut score_data = ScoreAccount::try_from_slice(&score_account.data.borrow())?;

    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Record the new loan
    score_data.record_new_loan(current_time);

    // Save changes
    score_data.serialize(&mut *score_account.data.borrow_mut())?;

    msg!("New loan recorded: total loans = {}", score_data.total_loans);
    Ok(())
}

pub struct ScoreContract;

impl ScoreContract {
    pub fn update_score(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        change: i16,
    ) -> ProgramResult {
        process_update_score(program_id, accounts, change)
    }
}
