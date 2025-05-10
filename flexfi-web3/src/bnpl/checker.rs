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
    // Vérifier si un utilisateur est autorisé à utiliser BNPL en fonction de son staking
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
        
        // Vérifier le compte de staking
        let seeds = [
            STAKING_SEED,
            user_account.key.as_ref(),
            usdc_mint.key.as_ref(),
        ];
        let (staking_pda, _) = Pubkey::find_program_address(&seeds, program_id);
        
        if *staking_account.key != staking_pda {
            return Err(ProgramError::InvalidAccountData);
        }
        
        // Charger les données de staking
        let staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;
        
        // Vérifier la propriété
        if staking_data.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }
        
        // Vérifier le statut du staking
        let status = staking_data.get_status()?;
        match status {
            StakingStatus::Frozen => {
                return Err(FlexfiError::StakingFrozen.into());
            },
            StakingStatus::Closed => {
                return Err(FlexfiError::StakingFrozen.into());
            },
            _ => {} // Active ou Locked sont OK
        }
        
        // Calculer le montant de staking requis (ratio 1:1)
        let required_staking = loan_amount;
        
        // Vérifier si le staking est suffisant
        if staking_data.amount_staked < required_staking {
            msg!("Staking insuffisant: a {}, besoin de {}", 
                 staking_data.amount_staked, required_staking);
            return Err(FlexfiError::InsufficientStaking.into());
        }
        
        // Vérifier le type de carte et les échéances autorisées
        let wallet_data = WalletAccount::try_from_slice(&wallet_account.data.borrow())?;
        
        if !wallet_data.is_active {
            return Err(FlexfiError::WalletInactive.into());
        }
        
        msg!("BNPL authorization successful: loan amount {}, staking {}", 
             loan_amount, staking_data.amount_staked);
        Ok(())
    }
    
    // Obtenir le montant maximum BNPL autorisé en fonction du staking
    pub fn get_max_bnpl_amount(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<u64, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        
        let staking_account = next_account_info(account_info_iter)?;
        let user_account = next_account_info(account_info_iter)?;
        
        // Charger les données de staking
        let staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;
        
        // Vérifier la propriété
        if staking_data.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }
        
        // Vérifier le statut du staking
        let status = staking_data.get_status()?;
        if status != StakingStatus::Active && status != StakingStatus::Locked {
            return Err(FlexfiError::StakingNotActive.into());
        }
        
        // Le montant maximum de BNPL est égal au montant staké (ratio 1:1)
        let max_bnpl = staking_data.amount_staked;
        
        msg!("Maximum BNPL amount: {}", max_bnpl);
        Ok(max_bnpl)
    }
    
    // Vérifier si le nombre d'échéances est autorisé pour ce type de carte
    pub fn check_installments_for_card(
        card_type: u8,
        installments: u8,
    ) -> Result<(), ProgramError> {
        let card_config = get_card_config(card_type);
        
        // Vérifier si le nombre d'échéances est autorisé pour ce type de carte
        let allowed = card_config.available_installments.contains(&installments);
        
        if !allowed {
            msg!("Échéance non autorisée: {} pour carte type {}", installments, card_type);
            return Err(FlexfiError::InvalidInstallmentForCard.into());
        }
        
        Ok(())
    }
}