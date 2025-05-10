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
    let user_status_account = next_account_info(account_info_iter)?; // Compte whitelist
    let user_token_account = next_account_info(account_info_iter)?;
    let vault_token_account = next_account_info(account_info_iter)?;
    let usdc_mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let associated_token_program = next_account_info(account_info_iter)?;
    let _rent_sysvar = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier la signature de l'utilisateur
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // VÉRIFIER QUE L'UTILISATEUR EST WHITELISTÉ
    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;
    
    // Vérifier le montant minimum
    if amount < MIN_STAKING_AMOUNT {
        return Err(FlexfiError::InsufficientStaking.into());
    }
    
    // Vérifier la période de verrouillage
    if lock_days < MIN_STAKING_LOCK_DAYS || lock_days > MAX_STAKING_LOCK_DAYS {
        return Err(ProgramError::InvalidArgument);
    }
    
    // Trouver le PDA du compte de staking
    let seeds = [
        STAKING_SEED,
        user_account.key.as_ref(),
        usdc_mint.key.as_ref(),
    ];
    let (staking_pda, staking_bump) = Pubkey::find_program_address(&seeds, program_id);
    
    if *staking_account.key != staking_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Trouver le PDA du vault
    let vault_seeds = [
        USDC_VAULT_SEED,
        staking_account.key.as_ref(),
    ];
    let (vault_pda, vault_bump) = Pubkey::find_program_address(&vault_seeds, program_id);
    
    if *vault_token_account.key != vault_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Obtenir l'horodatage actuel
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    
    // Initialiser ou mettre à jour le compte de staking
    let mut staking_data = if staking_account.owner == program_id {
        // Compte existant, charger les données
        let mut data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;
        
        // Vérifier que le staking est actif ou verrouillé (pas gelé ou fermé)
        let status = data.get_status()?;
        if status != StakingStatus::Active && status != StakingStatus::Locked {
            return Err(FlexfiError::StakingFrozen.into());
        }
        
        // Mettre à jour le montant et verrouiller si nécessaire
        data.amount_staked = data.amount_staked.saturating_add(amount);
        
        // Si l'utilisateur verrouille pour une période plus longue, mettre à jour
        if status == StakingStatus::Locked {
            let new_lock_end = current_time + (lock_days as i64 * 86400);
            if new_lock_end > data.lock_period_end {
                data.lock_period_end = new_lock_end;
            }
        } else {
            // Passer à l'état verrouillé
            data.set_status(StakingStatus::Locked);
            data.lock_period_end = current_time + (lock_days as i64 * 86400);
        }
        
        data.last_update = current_time;
        data
    } else {
        // Nouveau compte, créer d'abord le compte
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
        
        // Créer le vault de token si nécessaire
        if vault_token_account.owner != &spl_token::id() {
            // Utiliser associated_token_account ou créer un token account normal
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
        
        // Initialiser les données du staking
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
    
    // Sauvegarder les données du staking
    staking_data.serialize(&mut *staking_account.data.borrow_mut())?;
    
    // Transférer les tokens vers le vault
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
    let user_status_account = next_account_info(account_info_iter)?; // Compte whitelist
    let user_token_account = next_account_info(account_info_iter)?;
    let vault_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier la signature de l'utilisateur
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // VÉRIFIER QUE L'UTILISATEUR EST WHITELISTÉ
    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;
    
    // Charger les données du staking
    let mut staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;
    
    // Vérifier que l'utilisateur est le propriétaire
    if staking_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier l'état du staking
    let status = staking_data.get_status()?;
    if status == StakingStatus::Frozen || status == StakingStatus::Closed {
        return Err(FlexfiError::StakingFrozen.into());
    }
    
    // Obtenir l'horodatage actuel
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    
    // Si verrouillé, vérifier que la période de verrouillage est terminée
    if status == StakingStatus::Locked && current_time < staking_data.lock_period_end {
        return Err(FlexfiError::StakingFrozen.into());
    }
    
    // Vérifier que le montant demandé est disponible
    if amount > staking_data.amount_staked {
        return Err(FlexfiError::InsufficientStaking.into());
    }
    
    // Mettre à jour le montant staké
    staking_data.amount_staked = staking_data.amount_staked.saturating_sub(amount);
    staking_data.last_update = current_time;
    
    // Si le montant restant est inférieur au minimum, fermer le compte
    if staking_data.amount_staked < MIN_STAKING_AMOUNT {
        staking_data.set_status(StakingStatus::Closed);
    } else {
        // Sinon, passer à l'état actif
        staking_data.set_status(StakingStatus::Active);
    }
    
    // Sauvegarder les modifications
    staking_data.serialize(&mut *staking_account.data.borrow_mut())?;
    
    // Préparer les seeds pour signer avec le PDA du vault
    let vault_seeds = [
        USDC_VAULT_SEED,
        staking_account.key.as_ref(),
        &[staking_data.bump],
    ];
    
    // Transférer les tokens du vault vers l'utilisateur
    let transfer_ix = spl_token::instruction::transfer(
        token_program.key,
        vault_token_account.key,
        user_token_account.key,
        &staking_account.key, // Le compte de staking est l'autorité du vault
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
    let user_status_account = next_account_info(account_info_iter)?; // Compte whitelist
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier la signature de l'utilisateur
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // VÉRIFIER QUE L'UTILISATEUR EST WHITELISTÉ
    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;
    
    // Charger les données du staking
    let mut staking_data = StakingAccount::try_from_slice(&staking_account.data.borrow())?;
    
    // Vérifier que l'utilisateur est le propriétaire
    if staking_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier si le staking est verrouillé
    let status = staking_data.get_status()?;
    if status != StakingStatus::Locked {
        msg!("Staking is not locked");
        return Ok(());
    }
    
    // Obtenir l'horodatage actuel
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    
    // Vérifier si la période de verrouillage est terminée
    if current_time >= staking_data.lock_period_end {
        // Déverrouiller le staking
        staking_data.set_status(StakingStatus::Active);
        staking_data.last_update = current_time;
        
        // Sauvegarder les modifications
        staking_data.serialize(&mut *staking_account.data.borrow_mut())?;
        
        msg!("Staking unlocked: lock period has ended");
    } else {
        let remaining_time = staking_data.lock_period_end - current_time;
        let remaining_days = remaining_time / 86400;
        
        msg!("Staking still locked: {} days remaining", remaining_days);
    }
    
    Ok(())
}

// Gestionnaire pour les fonctions de staking
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