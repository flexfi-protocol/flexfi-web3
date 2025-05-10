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
use crate::state::wallet::WalletAccount;
use crate::state::card::CardAccount;
use crate::constants::{CARD_STANDARD, CARD_SILVER, CARD_GOLD, CARD_PLATINUM, CARD_SEED};
use crate::card::config::get_card_annual_fee;
use crate::core::whitelist::require_whitelisted;

pub fn process_upgrade_card(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_card_type: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let wallet_account = next_account_info(account_info_iter)?;
    let card_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let fee_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier la signature de l'utilisateur
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;
    
    // Vérifier que le type de carte est valide
    if new_card_type > CARD_PLATINUM {
        return Err(FlexfiError::InvalidCardType.into());
    }
    
    // Charger les données du wallet
    let mut wallet_data = WalletAccount::try_from_slice(&wallet_account.data.borrow())?;
    
    // Vérifier que l'utilisateur est le propriétaire du wallet
    if wallet_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier que le wallet est actif
    if !wallet_data.is_active {
        return Err(FlexfiError::WalletInactive.into());
    }
    
    // Vérifier que le nouveau type de carte est différent et supérieur
    if wallet_data.card_type == new_card_type {
        return Err(FlexfiError::AlreadyAtThisLevel.into());
    }
    
    if wallet_data.card_type > new_card_type {
        return Err(ProgramError::InvalidArgument);
    }
    
    // Calculer les frais d'upgrade
    let current_fee = get_card_annual_fee(wallet_data.card_type)?;
    let new_fee = get_card_annual_fee(new_card_type)?;
    
    let upgrade_fee = new_fee.saturating_sub(current_fee);
    
    // Obtenir l'horodatage actuel
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    
    // Créer ou mettre à jour le compte de carte
    if card_account.owner == program_id {
        // Mettre à jour la carte existante
        let mut card_data = CardAccount::try_from_slice(&card_account.data.borrow())?;
        
        // Vérifier que l'utilisateur est le propriétaire
        if card_data.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }
        
        // Mettre à jour le type de carte
        card_data.card_type = new_card_type;
        
        // Mettre à jour la date d'expiration des frais annuels
        card_data.annual_fee_paid_until = current_time + (365 * 86400);
        
        // Sauvegarder les modifications
        card_data.serialize(&mut *card_account.data.borrow_mut())?;
    } else {
        // Créer un nouveau compte de carte
        let seeds = [
            CARD_SEED,
            user_account.key.as_ref(),
        ];
        let (card_pda, card_bump) = Pubkey::find_program_address(&seeds, program_id);
        
        if *card_account.key != card_pda {
            return Err(ProgramError::InvalidAccountData);
        }
        
        // Créer le compte
        let rent = Rent::get()?;
        let space = CardAccount::SIZE;
        let rent_lamports = rent.minimum_balance(space);
        
        invoke_signed(
            &system_instruction::create_account(
                user_account.key,
                &card_pda,
                rent_lamports,
                space as u64,
                program_id,
            ),
            &[user_account.clone(), card_account.clone(), system_program.clone()],
            &[&[CARD_SEED, user_account.key.as_ref(), &[card_bump]]],
        )?;
        
        // Initialiser les données de carte
        let card_data = CardAccount::new(
            *user_account.key,
            new_card_type,
            current_time,
            card_bump,
        );
        
        // Sauvegarder les données
        card_data.serialize(&mut *card_account.data.borrow_mut())?;
    }
    
    // Mettre à jour le type de carte dans le wallet
    wallet_data.card_type = new_card_type;
    wallet_data.serialize(&mut *wallet_account.data.borrow_mut())?;
    
    // Transférer les frais d'upgrade si nécessaire
    if upgrade_fee > 0 {
        let transfer_ix = spl_token::instruction::transfer(
            token_program.key,
            user_token_account.key,
            fee_account.key,
            user_account.key,
            &[],
            upgrade_fee,
        )?;
        
        invoke(
            &transfer_ix,
            &[
                user_token_account.clone(),
                fee_account.clone(),
                user_account.clone(),
                token_program.clone(),
            ],
        )?;
    }
    
    msg!("Card upgraded to type {}", new_card_type);
    Ok(())
}

pub struct CardManager;

impl CardManager {
    pub fn upgrade_card(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_card_type: u8,
    ) -> ProgramResult {
        process_upgrade_card(program_id, accounts, new_card_type)
    }
}