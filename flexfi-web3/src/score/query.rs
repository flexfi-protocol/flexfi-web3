use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    msg,
};
use borsh::BorshDeserialize;

use crate::error::FlexfiError;
use crate::state::score::ScoreAccount;
use crate::constants::SCORE_SEED;

pub fn process_get_score(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let score_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;

    // Check the score account
    let seeds = [
        SCORE_SEED,
        user_account.key.as_ref(),
    ];
    let (score_pda, _) = Pubkey::find_program_address(&seeds, program_id);

    if *score_account.key != score_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Load score data
    let score_data = ScoreAccount::try_from_slice(&score_account.data.borrow())?;

    // Verify ownership
    if score_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Display score information
    msg!("User score: {}", score_data.score);
    msg!("On-time payments: {}", score_data.on_time_payments);
    msg!("Late payments: {}", score_data.late_payments);
    msg!("Defaults: {}", score_data.defaults);
    msg!("Total loans: {}", score_data.total_loans);

    Ok(())
}

pub fn process_check_score_threshold(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    min_score: u16,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let score_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;

    // Check the score account
    let seeds = [
        SCORE_SEED,
        user_account.key.as_ref(),
    ];
    let (score_pda, _) = Pubkey::find_program_address(&seeds, program_id);

    if *score_account.key != score_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Load score data
    let score_data = ScoreAccount::try_from_slice(&score_account.data.borrow())?;

    // Verify ownership
    if score_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Check if the score meets the minimum threshold
    let meets_threshold = score_data.score >= min_score;

    msg!("Score check: user score {} vs threshold {}: {}",
         score_data.score, min_score, meets_threshold);

    Ok(())
}

pub fn process_get_payment_stats(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let score_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;

    // Check the score account
    let seeds = [
        SCORE_SEED,
        user_account.key.as_ref(),
    ];
    let (score_pda, _) = Pubkey::find_program_address(&seeds, program_id);

    if *score_account.key != score_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Load score data
    let score_data = ScoreAccount::try_from_slice(&score_account.data.borrow())?;

    // Verify ownership
    if score_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Calculate statistics
    let total_payments = score_data.on_time_payments + score_data.late_payments;
    let on_time_percentage = if total_payments > 0 {
        (score_data.on_time_payments as f64 / total_payments as f64) * 100.0
    } else {
        0.0
    };

    // Display statistics
    msg!("Payment statistics:");
    msg!("Total payments: {}", total_payments);
    msg!("On-time payments: {} ({}%)",
         score_data.on_time_payments, on_time_percentage);
    msg!("Late payments: {}", score_data.late_payments);
    msg!("Defaults: {}", score_data.defaults);
    msg!("Total loans: {}", score_data.total_loans);

    Ok(())
}

pub struct ScoreQuery;

impl ScoreQuery {
    pub fn get_score(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        process_get_score(program_id, accounts)
    }
}
