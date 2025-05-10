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
use crate::state::{
    bnpl::{BNPLContractAccount, BNPLStatus},
    wallet::WalletAccount,
    nft::{NFTMetadataAccount, NFTType},
    staking::StakingAccount,
};
use crate::constants::{
    BNPL_CONTRACT_SEED, NFT_NONE,
    get_card_config, get_nft_apr_bonus,
    MIN_BNPL_INSTALLMENTS, MAX_BNPL_INSTALLMENTS,
    MIN_PAYMENT_INTERVAL_DAYS, MAX_PAYMENT_INTERVAL_DAYS
};
use crate::bnpl::checker::BNPLChecker;

pub fn process_create_bnpl_contract(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    installments: u8,
    payment_interval_days: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let bnpl_account = next_account_info(account_info_iter)?;
    let borrower_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let merchant_account = next_account_info(account_info_iter)?;
    let staking_account = next_account_info(account_info_iter)?;
    let wallet_account = next_account_info(account_info_iter)?;
    let usdc_mint = next_account_info(account_info_iter)?;
    let borrower_token_account = next_account_info(account_info_iter)?;
    let merchant_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier signature du borrower
    if !borrower_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    require_whitelisted(
        program_id,
        borrower_account.key,
        user_status_account
    )?;    

    // Vérifier que le nombre d'échéances est valide
    if installments < MIN_BNPL_INSTALLMENTS || installments > MAX_BNPL_INSTALLMENTS {
        return Err(FlexfiError::InvalidInstallments.into());
    }
    
    // Vérifier que l'intervalle de paiement est valide
    if payment_interval_days < MIN_PAYMENT_INTERVAL_DAYS || payment_interval_days > MAX_PAYMENT_INTERVAL_DAYS {
        return Err(ProgramError::InvalidArgument);
    }
        // NOUVEAU : Vérifier que l'utilisateur est whitelisté
    require_whitelisted(
        program_id,
        borrower_account.key,
        user_status_account
    )?;
    
    // Vérifier avec BNPLChecker que l'utilisateur est autorisé
    BNPLChecker::check_bnpl_authorization(program_id, &[
        staking_account.clone(),
        borrower_account.clone(),
        usdc_mint.clone(),
        wallet_account.clone(),
    ], amount)?;
    
    // Récupérer le type de carte depuis le wallet
    let wallet_data = WalletAccount::try_from_slice(&wallet_account.data.borrow())?;
    let card_type = wallet_data.card_type;
    
    // Vérifier si le nombre d'échéances est autorisé pour ce type de carte
    BNPLChecker::check_installments_for_card(card_type, installments)?;
    
    // Récupérer la configuration de la carte
    let card_config = get_card_config(card_type);
    
    // Vérifier le NFT attaché (si présent)
    let nft_type = if account_info_iter.len() > 0 {
        let nft_account = next_account_info(account_info_iter)?;
        
        if !nft_account.data_is_empty() {
            let nft_data = NFTMetadataAccount::try_from_slice(&nft_account.data.borrow())?;
            
            // Vérifier que le NFT est actif et appartient à l'utilisateur
            if nft_data.owner != *borrower_account.key || !nft_data.is_active {
                NFT_NONE
            } else {
                // Vérifier si le NFT est expiré
                let clock = Clock::from_account_info(clock_sysvar)?;
                let current_time = clock.unix_timestamp;
                
                if nft_data.is_expired(current_time) {
                    NFT_NONE
                } else {
                    nft_data.nft_type
                }
            }
        } else {
            NFT_NONE
        }
    } else {
        NFT_NONE
    };
    
    // Créer un identifiant unique pour ce contrat
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_timestamp = clock.unix_timestamp;
    
    let contract_seed = [
        BNPL_CONTRACT_SEED,
        borrower_account.key.as_ref(),
        merchant_account.key.as_ref(),
        &current_timestamp.to_le_bytes(),
    ].concat();
    
    let (bnpl_pda, bnpl_bump) = Pubkey::find_program_address(&[&contract_seed[..]], program_id);
    
    if *bnpl_account.key != bnpl_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Créer le compte BNPL
    let rent = Rent::get()?;
    let space = BNPLContractAccount::SIZE;
    let rent_lamports = rent.minimum_balance(space);
    
    msg!("Creating BNPL contract account with size: {}", space);
    
    invoke_signed(
        &system_instruction::create_account(
            borrower_account.key,
            &bnpl_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[borrower_account.clone(), bnpl_account.clone(), system_program.clone()],
        &[&[&contract_seed[..], &[bnpl_bump]]],
    )?;
    
    // Calculer les frais BNPL
    let fee_percentage = if installments == 12 {
        card_config.bnpl_fee_12months
    } else {
        card_config.bnpl_fee_percentage
    };
    
    // Calculer l'APR avec bonus NFT
    let apr_percentage = card_config.apr_percentage + get_nft_apr_bonus(nft_type);
    
    // Calculer le montant des frais
    let fee_amount = amount
        .checked_mul(fee_percentage as u64)
        .ok_or(FlexfiError::MathOverflow)?
        .checked_div(10000)
        .ok_or(FlexfiError::MathOverflow)?;
    
    // Calculer le montant des intérêts (APR)
    let apr_amount = amount
        .checked_mul(apr_percentage as u64)
        .ok_or(FlexfiError::MathOverflow)?
        .checked_div(10000)
        .ok_or(FlexfiError::MathOverflow)?
        .checked_mul(installments as u64)
        .ok_or(FlexfiError::MathOverflow)?
        .checked_div(12) // Divisé par 12 mois pour un taux mensuel
        .ok_or(FlexfiError::MathOverflow)?;
    
    // Calculer le montant total (principal + frais + intérêts)
    let total_amount = amount
        .checked_add(fee_amount)
        .ok_or(FlexfiError::MathOverflow)?
        .checked_add(apr_amount)
        .ok_or(FlexfiError::MathOverflow)?;
    
    // Calculer le montant par échéance
    let amount_per_installment = total_amount
        .checked_div(installments as u64)
        .ok_or(FlexfiError::MathOverflow)?;
    
    // Calculer la date de la prochaine échéance
    let next_payment_due = current_timestamp + (payment_interval_days as i64 * 86400);
    
    // Initialiser le contrat BNPL
    let bnpl_contract = BNPLContractAccount::new(
        *borrower_account.key,
        *merchant_account.key,
        amount,
        *usdc_mint.key,
        installments,
        payment_interval_days,
        amount_per_installment,
        fee_percentage,
        apr_percentage,
        card_type,
        nft_type,
        current_timestamp,
        next_payment_due,
        bnpl_bump,
    );
    
    // Sauvegarder le contrat
    bnpl_contract.serialize(&mut *bnpl_account.data.borrow_mut())?;
    
    // Transférer les fonds du prêteur (programme) au marchand
    let transfer_ix = spl_token::instruction::transfer(
        token_program.key,
        borrower_token_account.key,
        merchant_token_account.key,
        borrower_account.key,
        &[],
        amount,
    )?;
    
    invoke(
        &transfer_ix,
        &[
            borrower_token_account.clone(),
            merchant_token_account.clone(),
            borrower_account.clone(),
            token_program.clone(),
        ],
    )?;
    
    msg!("BNPL contract created: amount={}, installments={}, fee={}%, apr={}%", 
         amount, installments, fee_percentage as f64 / 100.0, apr_percentage as f64 / 100.0);
    Ok(())
}

pub fn process_make_bnpl_payment(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let bnpl_account = next_account_info(account_info_iter)?;
    let borrower_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let borrower_token_account = next_account_info(account_info_iter)?;
    let platform_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier signature du borrower
    if !borrower_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    require_whitelisted(
        _program_id,
        borrower_account.key,
        user_status_account
    )?;
    
    // Charger les données du contrat BNPL
    let mut bnpl_contract = BNPLContractAccount::try_from_slice(&bnpl_account.data.borrow())?;
    
    // Vérifier que le contrat appartient au borrower
    if bnpl_contract.borrower != *borrower_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier que le contrat est actif
    let status = bnpl_contract.get_status()?;
    if status != BNPLStatus::Active {
        return Err(FlexfiError::LoanNotActive.into());
    }
    
    // Vérifier si toutes les échéances sont déjà payées
    if bnpl_contract.paid_installments >= bnpl_contract.installments {
        bnpl_contract.set_status(BNPLStatus::Completed);
        bnpl_contract.serialize(&mut *bnpl_account.data.borrow_mut())?;
        return Ok(());
    }
    
    // Transférer le paiement
    let transfer_ix = spl_token::instruction::transfer(
        token_program.key,
        borrower_token_account.key,
        platform_token_account.key,
        borrower_account.key,
        &[],
        bnpl_contract.amount_per_installment,
    )?;
    
    invoke(
        &transfer_ix,
        &[
            borrower_token_account.clone(),
            platform_token_account.clone(),
            borrower_account.clone(),
            token_program.clone(),
        ],
    )?;
    
    // Mettre à jour le contrat
    let clock = Clock::from_account_info(clock_sysvar)?;
    bnpl_contract.update_after_payment(clock.unix_timestamp)?;
    
    // Sauvegarder les modifications
    bnpl_contract.serialize(&mut *bnpl_account.data.borrow_mut())?;
    
    if bnpl_contract.get_status()? == BNPLStatus::Completed {
        msg!("BNPL contract completed: all installments paid");
    } else {
        msg!("Payment made: {}/{} installments", 
             bnpl_contract.paid_installments, bnpl_contract.installments);
    }
    
    Ok(())
}

pub fn process_cancel_bnpl_contract(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let bnpl_account = next_account_info(account_info_iter)?;
    let borrower_account = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier signature du borrower
    if !borrower_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Charger les données du contrat BNPL
    let mut bnpl_contract = BNPLContractAccount::try_from_slice(&bnpl_account.data.borrow())?;
    
    // Vérifier que le contrat appartient au borrower
    if bnpl_contract.borrower != *borrower_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier que le contrat est actif
    let status = bnpl_contract.get_status()?;
    if status != BNPLStatus::Active {
        return Err(FlexfiError::LoanNotActive.into());
    }
    
    // Vérifier qu'aucun paiement n'a été effectué
    if bnpl_contract.paid_installments > 0 {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Marquer le contrat comme annulé
    bnpl_contract.set_status(BNPLStatus::Cancelled);
    
    // Mettre à jour la date de dernière modification
    let clock = Clock::from_account_info(clock_sysvar)?;
    bnpl_contract.last_payment_at = clock.unix_timestamp;
    
    // Sauvegarder les modifications
    bnpl_contract.serialize(&mut *bnpl_account.data.borrow_mut())?;
    
    msg!("BNPL contract cancelled");
    Ok(())
}

pub struct BNPLContract;

impl BNPLContract {
    pub fn create(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        installments: u8,
        payment_interval_days: u8,
    ) -> ProgramResult {
        process_create_bnpl_contract(program_id, accounts, amount, installments, payment_interval_days)
    }
}