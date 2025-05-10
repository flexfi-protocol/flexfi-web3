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
use crate::state::wallet::WalletAccount;  // Import correct
use crate::constants::{WALLET_SEED, CARD_PLATINUM};
use crate::core::whitelist::check_user_whitelisted;  // Import de la fonction

pub fn process_create_wallet(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    card_type: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let wallet_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?; // Nouveau : compte de whitelist
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier la signature de l'utilisateur
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier que l'utilisateur est dans la whitelist
    let is_whitelisted = check_user_whitelisted(
        program_id,
        user_account.key,
        &[user_status_account.clone()]
    )?;
    
    if !is_whitelisted {
        msg!("User {} is not whitelisted", user_account.key);
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier que le type de carte est valide
    if card_type > CARD_PLATINUM {
        return Err(FlexfiError::InvalidCardType.into());
    }
    
    // Créer un PDA pour ce wallet basé sur la clé publique de l'utilisateur
    let seeds = [
        WALLET_SEED,
        user_account.key.as_ref(),
    ];
    let (wallet_pda, bump_seed) = Pubkey::find_program_address(&seeds, program_id);
    
    if *wallet_account.key != wallet_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Créer le compte wallet
    let rent = Rent::get()?;
    let space = WalletAccount::SIZE;
    let rent_lamports = rent.minimum_balance(space);
    
    invoke_signed(
        &system_instruction::create_account(
            user_account.key,
            &wallet_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[user_account.clone(), wallet_account.clone(), system_program.clone()],
        &[&[WALLET_SEED, user_account.key.as_ref(), &[bump_seed]]],
    )?;
    
    // Obtenir l'horodatage actuel
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    
    // Initialiser les données du wallet (sans backend_id)
    let wallet_data = WalletAccount::new(
        *user_account.key,
        card_type,
        current_time,
        bump_seed,
    );
    
    wallet_data.serialize(&mut *wallet_account.data.borrow_mut())?;
    
    msg!("Wallet created for whitelisted user: {:?}", user_account.key);
    Ok(())
}

pub fn process_deactivate_wallet(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let wallet_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    
    // Vérifier la signature de l'utilisateur
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Charger les données du wallet
    let mut wallet_data = WalletAccount::try_from_slice(&wallet_account.data.borrow())?;
    
    // Vérifier que l'utilisateur est le propriétaire
    if wallet_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Désactiver le wallet
    wallet_data.is_active = false;
    
    // Sauvegarder les modifications
    wallet_data.serialize(&mut *wallet_account.data.borrow_mut())?;
    
    msg!("Wallet deactivated: {:?}", wallet_account.key);
    Ok(())
}

pub fn process_reactivate_wallet(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let wallet_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let admin_account = next_account_info(account_info_iter)?;
    
    // Vérifier les signatures
    if !user_account.is_signer || !admin_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Charger les données du wallet
    let mut wallet_data = WalletAccount::try_from_slice(&wallet_account.data.borrow())?;
    
    // Vérifier que l'utilisateur est le propriétaire
    if wallet_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Réactiver le wallet
    wallet_data.is_active = true;
    
    // Sauvegarder les modifications
    wallet_data.serialize(&mut *wallet_account.data.borrow_mut())?;
    
    msg!("Wallet reactivated: {:?}", wallet_account.key);
    Ok(())
}