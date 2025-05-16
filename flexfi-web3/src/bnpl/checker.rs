use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    msg,
};
use borsh::BorshDeserialize;

use crate::error::FlexfiError;
use crate::state::{staking::{StakingAccount, StakingStatus}, wallet::WalletAccount};
use crate::constants::{STAKING_SEED, get_card_config};

pub struct BNPLChecker {}

impl BNPLChecker {
    // Check if a user is authorized to use BNPL based on their staking
    pub fn check_bnpl_authorization(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        loan_amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let staking_account = next_account_info(account_info_iter)?;
        let user_account = next_account_info(account_info_iter)?;
        let usdc_mint = next_account_info(account_info_iter)?;
        let wallet_account = next_account_info(account_info_iter)?;

        // Check the staking account
        let seeds = [
            STAKING_SEED,
            user_account.key.as_ref(),
            usdc_mint.key.as_ref(),
        ];
        let (staking_pda, _) = Pubkey::find_program_address(&seeds, program_id);

        if *staking_account.key != staking_pda {
            return Err(ProgramError::InvalidAccountData);
        }

        // Load staking data
        let staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;

        // Verify ownership
        if staking_data.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }

        // Check staking status
        let status = staking_data.get_status()?;
        match status {
            StakingStatus::Frozen => {
                return Err(FlexfiError::StakingFrozen.into());
            },
            StakingStatus::Closed => {
                return Err(FlexfiError::StakingFrozen.into());
            },
            _ => {} // Active or Locked are OK
        }

        // Calculate required staking amount (1:1 ratio)
        let required_staking = loan_amount;

        // Check if staking is sufficient
        if staking_data.amount_staked < required_staking {
            msg!("Insufficient staking: has {}, needs {}", staking_data.amount_staked, required_staking);
            return Err(FlexfiError::InsufficientStaking.into());
        }

        // Check card type and allowed installments
        let wallet_data = WalletAccount::try_from_slice(&wallet_account.data.borrow())?;

        if !wallet_data.is_active {
            return Err(FlexfiError::WalletInactive.into());
        }

        msg!("BNPL authorization successful: loan amount {}, staking {}", loan_amount, staking_data.amount_staked);
        Ok(())
    }

    // Get the maximum BNPL amount allowed based on staking
    pub fn get_max_bnpl_amount(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<u64, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let staking_account = next_account_info(account_info_iter)?;
        let user_account = next_account_info(account_info_iter)?;

        // Load staking data
        let staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;

        // Verify ownership
        if staking_data.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }

        // Check staking status
        let status = staking_data.get_status()?;
        if status != StakingStatus::Active && status != StakingStatus::Locked {
            return Err(FlexfiError::StakingNotActive.into());
        }

        // The maximum BNPL amount is equal to the staked amount (1:1 ratio)
        let max_bnpl = staking_data.amount_staked;

        msg!("Maximum BNPL amount: {}", max_bnpl);
        Ok(max_bnpl)
    }

    // Check if the number of installments is allowed for this card type
    pub fn check_installments_for_card(
        card_type: u8,
        installments: u8,
    ) -> Result<(), ProgramError> {
        let card_config = get_card_config(card_type);

        // Check if the number of installments is allowed for this card type
        let allowed = card_config.available_installments.contains(&installments);

        if !allowed {
            msg!("Installment not allowed: {} for card type {}", installments, card_type);
            return Err(FlexfiError::InvalidInstallmentForCard.into());
        }

        Ok(())
    }
}