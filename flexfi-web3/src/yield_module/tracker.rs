use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
    msg,
};
use borsh::{BorshDeserialize, BorshSerialize};
use crate::core::whitelist::require_whitelisted;
use crate::error::FlexfiError;
use crate::state::yield_::YieldAccount;

pub fn process_claim_yield(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let yield_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let yield_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier la signature de l'utilisateur
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    require_whitelisted(
        _program_id,
        user_account.key,
        user_status_account
    )?;
    
    // Charger les données du yield
    let mut yield_data = YieldAccount::try_from_slice(&yield_account.data.borrow())?;
    
    // Vérifier la propriété
    if yield_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier si le montant demandé est disponible
    let unclaimed_yield = yield_data.get_unclaimed_yield();
    if amount > unclaimed_yield {
        return Err(FlexfiError::NoYieldToClaim.into());
    }
    
    // Obtenir l'horodatage actuel
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    
    // Si auto_reinvest est activé et que le montant est inférieur à un seuil,
    // réinvestir automatiquement
    if yield_data.auto_reinvest && amount < 1_000_000 {
        // Réinvestir automatiquement (logique simplifiée)
        yield_data.record_yield_claimed(amount, current_time)?;
        yield_data.record_yield_earned(amount);
        
        msg!("Yield auto-reinvested: {}", amount);
    } else {
        // Transférer le yield depuis le compte de yield vers le compte de l'utilisateur
        let transfer_ix = spl_token::instruction::transfer(
            token_program.key,
            yield_token_account.key,
            user_token_account.key,
            yield_account.key, // L'autorité est le PDA du yield
            &[],
            amount,
        )?;
        
        // Obtenir les seeds pour signer
        let seeds = [
            b"yield_config",
            user_account.key.as_ref(),
            &[yield_data.bump],
        ];
        
        solana_program::program::invoke_signed(
            &transfer_ix,
            &[
                yield_token_account.clone(),
                user_token_account.clone(),
                yield_account.clone(),
                token_program.clone(),
            ],
            &[&seeds],
        )?;
        
        // Enregistrer le yield réclamé
        yield_data.record_yield_claimed(amount, current_time)?;
        
        msg!("Yield claimed: {}", amount);
    }
    
    // Sauvegarder les modifications
    yield_data.serialize(&mut *yield_account.data.borrow_mut())?;
    
    Ok(())
}

pub fn process_get_yield_stats(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let yield_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    
    // Charger les données du yield
    let yield_data = YieldAccount::try_from_slice(&yield_account.data.borrow())?;
    
    // Vérifier la propriété
    if yield_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Afficher les statistiques
    msg!("Yield statistics:");
    msg!("Total yield earned: {}", yield_data.total_yield_earned);
    msg!("Total yield claimed: {}", yield_data.total_yield_claimed);
    msg!("Unclaimed yield: {}", yield_data.get_unclaimed_yield());
    msg!("Strategy: {:?}", yield_data.get_strategy()?);
    msg!("Auto-reinvest: {}", yield_data.auto_reinvest);
    
    Ok(())
}

pub struct YieldTracker;

impl YieldTracker {
    pub fn claim_yield(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        process_claim_yield(program_id, accounts, amount)
    }
}