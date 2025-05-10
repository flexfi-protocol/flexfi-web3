use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
    msg,
};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::error::FlexfiError;
use crate::state::{
    bnpl::{BNPLContractAccount, BNPLStatus},
    staking::StakingAccount,
    wallet::WalletAccount,
};
use crate::constants::{GRACE_PERIOD_DAYS, USDC_VAULT_SEED, get_late_payment_penalty};
use crate::score::contract::ScoreContract;

pub fn process_check_repayment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let bnpl_account = next_account_info(account_info_iter)?;
    let borrower_token_account = next_account_info(account_info_iter)?;
    let platform_token_account = next_account_info(account_info_iter)?;
    let borrower_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let score_account = next_account_info(account_info_iter)?;
    let staking_account = next_account_info(account_info_iter)?;
    let staking_token_account = next_account_info(account_info_iter)?;
    let wallet_account = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Charger les données du contrat BNPL
    let mut bnpl_contract = BNPLContractAccount::try_from_slice(&bnpl_account.data.borrow())?;
    
    // Vérifier que le contrat est actif
    let status = bnpl_contract.get_status()?;
    if status != BNPLStatus::Active {
        return Ok(());
    }
    
    // Vérifier si le paiement est dû
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_timestamp = clock.unix_timestamp;
    
    if !bnpl_contract.is_payment_due(current_timestamp) {
        return Ok(());
    }
    
    // D'abord essayer de prélever depuis le compte de l'utilisateur
    let transfer_ix = spl_token::instruction::transfer(
        token_program.key,
        borrower_token_account.key,
        platform_token_account.key,
        borrower_account.key,
        &[],
        bnpl_contract.amount_per_installment,
    )?;
    
    let result = invoke(
        &transfer_ix,
        &[
            borrower_token_account.clone(),
            platform_token_account.clone(),
            borrower_account.clone(),
            token_program.clone(),
        ],
    );
    
    // Si le prélèvement réussit, mettre à jour le contrat
    if result.is_ok() {
        // Mettre à jour l'état du contrat
        bnpl_contract.update_after_payment(current_timestamp)?;
        
        // Mettre à jour le score (bonus pour paiement à l'heure)
        ScoreContract::update_score(
            program_id,
            &[
                score_account.clone(),
                borrower_account.clone(),
            ],
            5, // +5 points pour paiement à l'heure
        )?;
        
        // Sauvegarder l'état du contrat
        bnpl_contract.serialize(&mut *bnpl_account.data.borrow_mut())?;
        
        if bnpl_contract.get_status()? == BNPLStatus::Completed {
            // Bonus de score pour contrat complété
            ScoreContract::update_score(
                program_id,
                &[
                    score_account.clone(),
                    borrower_account.clone(),
                ],
                20, // +20 points pour contrat complété
            )?;
            
            msg!("BNPL contract completed: all installments paid");
        } else {
            msg!("Payment processed: {}/{} installments", 
                 bnpl_contract.paid_installments, bnpl_contract.installments);
        }
        
        return Ok(());
    }
    
    // Si le paiement a échoué, vérifier si le délai de grâce est dépassé
    let grace_period = GRACE_PERIOD_DAYS as i64 * 86400; // 15 jours en secondes
    
    if current_timestamp > bnpl_contract.next_payment_due + grace_period {
        // Prélever depuis le staking (déstaking automatique)
        
        // Charger les données du wallet et du staking
        let wallet_data = WalletAccount::try_from_slice(&wallet_account.data.borrow())?;
        let mut staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;
        
        // Vérifier que le staking appartient à l'emprunteur
        if staking_data.owner != *borrower_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }
        
        // Récupérer le type de NFT depuis le contrat BNPL
        let nft_type = bnpl_contract.nft_type;
        
        // Calculer la pénalité selon la combinaison carte+NFT
        let penalty_percentage = get_late_payment_penalty(wallet_data.card_type, nft_type);
        
        let penalty_amount = bnpl_contract.amount_per_installment
            .checked_mul(penalty_percentage as u64)
            .ok_or(FlexfiError::MathOverflow)?
            .checked_div(10000)
            .ok_or(FlexfiError::MathOverflow)?;
        
        let total_deduction = bnpl_contract.amount_per_installment
            .checked_add(penalty_amount)
            .ok_or(FlexfiError::MathOverflow)?;
        
        // Vérifier que le staking est suffisant
        if staking_data.amount_staked < total_deduction {
            // Utiliser tout le staking disponible
            let available_amount = staking_data.amount_staked;
            
            if available_amount == 0 {
                // Mettre à jour le score (forte pénalité pour défaut sans collatéral)
                ScoreContract::update_score(
                    program_id,
                    &[
                        score_account.clone(),
                        borrower_account.clone(),
                    ],
                    -50, // -50 points pour défaut de paiement
                )?;
                
                // Marquer le contrat comme défaillant
                bnpl_contract.set_status(BNPLStatus::Defaulted);
                bnpl_contract.serialize(&mut *bnpl_account.data.borrow_mut())?;
                
                msg!("BNPL contract defaulted: no collateral available");
                return Err(FlexfiError::InsufficientCollateralForAutoDebit.into());
            }
            
            // Mettre à jour le montant de staking
            staking_data.amount_staked = 0;
            
            // Préparer les seeds pour signer avec le PDA du vault
            let vault_seeds = [
                USDC_VAULT_SEED,
                staking_account.key.as_ref(),
                &[staking_data.bump],
            ];
            
            // Transférer le montant disponible
            let transfer_stake_ix = spl_token::instruction::transfer(
                token_program.key,
                staking_token_account.key,
                platform_token_account.key,
                staking_account.key, // Le compte de staking est l'autorité du vault
                &[],
                available_amount,
            )?;
            
            invoke_signed(
                &transfer_stake_ix,
                &[
                    staking_token_account.clone(),
                    platform_token_account.clone(),
                    staking_account.clone(),
                    token_program.clone(),
                ],
                &[&vault_seeds],
            )?;
            
            // Si le montant disponible couvre au moins l'échéance (sans pénalité)
            if available_amount >= bnpl_contract.amount_per_installment {
                // Mettre à jour le contrat BNPL
                bnpl_contract.update_after_payment(current_timestamp)?;
                
                // Mettre à jour le score (pénalité mais pas default)
                ScoreContract::update_score(
                    program_id,
                    &[
                        score_account.clone(),
                        borrower_account.clone(),
                    ],
                    -20, // -20 points pour paiement en retard avec déstaking
                )?;
            } else {
                // Marquer le contrat comme défaillant
                bnpl_contract.set_status(BNPLStatus::Defaulted);
                
                // Mettre à jour le score (forte pénalité)
                ScoreContract::update_score(
                    program_id,
                    &[
                        score_account.clone(),
                        borrower_account.clone(),
                    ],
                    -50, // -50 points pour défaut de paiement
                )?;
            }
        } else {
            // Réduire le montant du staking
            staking_data.amount_staked = staking_data.amount_staked
                .checked_sub(total_deduction)
                .ok_or(FlexfiError::MathOverflow)?;
            
            // Préparer les seeds pour signer avec le PDA du vault
            let vault_seeds = [
                USDC_VAULT_SEED,
                staking_account.key.as_ref(),
                &[staking_data.bump],
            ];
            
            // Transférer les tokens du staking
            let transfer_stake_ix = spl_token::instruction::transfer(
                token_program.key,
                staking_token_account.key,
                platform_token_account.key,
                staking_account.key, // Le compte de staking est l'autorité du vault
                &[],
                total_deduction,
            )?;
            
            invoke_signed(
                &transfer_stake_ix,
                &[
                    staking_token_account.clone(),
                    platform_token_account.clone(),
                    staking_account.clone(),
                    token_program.clone(),
                ],
                &[&vault_seeds],
            )?;
            
            // Mettre à jour le contrat BNPL
            bnpl_contract.update_after_payment(current_timestamp)?;
            
            // Mettre à jour le score (pénalité pour retard)
            ScoreContract::update_score(
                program_id,
                &[
                    score_account.clone(),
                    borrower_account.clone(),
                ],
                -20, // -20 points pour paiement en retard avec déstaking
            )?;
        }
        
        // Sauvegarder les modifications
        staking_data.serialize(&mut *staking_account.data.borrow_mut())?;
        bnpl_contract.serialize(&mut *bnpl_account.data.borrow_mut())?;
        
        msg!("Payment processed from staking with penalty of {}%", 
             penalty_percentage as f64 / 100.0);
    } else {
        // Le paiement est en retard mais dans le délai de grâce
        // Mettre à jour le score (petite pénalité)
        ScoreContract::update_score(
            program_id,
            &[
                score_account.clone(),
                borrower_account.clone(),
            ],
            -10, // -10 points pour retard dans le délai de grâce
        )?;
        
        msg!("Payment overdue but within grace period");
    }
    
    Ok(())
}

pub struct RepaymentChecker;

impl RepaymentChecker {
    pub fn check_repayment(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        process_check_repayment(program_id, accounts)
    }
}