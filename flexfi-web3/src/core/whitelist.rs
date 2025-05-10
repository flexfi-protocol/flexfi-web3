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
use crate::state::whitelist::{WhitelistAccount, UserWhitelistStatus};
use crate::constants::{WHITELIST_SEED, HARDCODED_WHITELIST};

// Vérifier si un utilisateur est dans la liste hardcodée (temporaire)
pub fn is_hardcoded_whitelisted(user_pubkey: &Pubkey) -> bool {
    let user_key_str = user_pubkey.to_string();
    HARDCODED_WHITELIST.contains(&user_key_str.as_str())
}

// Vérifier si un utilisateur est whitelisté (hardcodé ou on-chain)
pub fn check_user_whitelisted(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    accounts: &[AccountInfo],
) -> Result<bool, ProgramError> {
    // D'abord vérifier la liste hardcodée
    if is_hardcoded_whitelisted(user_pubkey) {
        msg!("User {} is hardcoded whitelisted", user_pubkey);
        return Ok(true);
    }
    
    // Ensuite vérifier la whitelist on-chain
    let account_info_iter = &mut accounts.iter();
    let user_status_account = next_account_info(account_info_iter)?;
    
    // Vérifier le PDA
    let (user_status_pda, _) = Pubkey::find_program_address(
        &[WHITELIST_SEED, user_pubkey.as_ref()],
        program_id
    );
    
    if user_status_account.key != &user_status_pda {
        return Ok(false);
    }
    
    // Si le compte n'existe pas, l'utilisateur n'est pas whitelisté
    if user_status_account.data_is_empty() {
        return Ok(false);
    }
    
    // Charger et vérifier le statut
    let user_status = UserWhitelistStatus::try_from_slice(&user_status_account.data.borrow())?;
    
    Ok(user_status.is_whitelisted)
}

// Fonction helper qui génère une erreur si l'utilisateur n'est pas whitelisté
pub fn require_whitelisted(
    program_id: &Pubkey,
    user_pubkey: &Pubkey,
    user_status_account: &AccountInfo,
) -> ProgramResult {
    let is_whitelisted = check_user_whitelisted(
        program_id,
        user_pubkey,
        &[user_status_account.clone()]
    )?;
    
    if !is_whitelisted {
        msg!("User {} is not whitelisted and cannot use this function", user_pubkey);
        return Err(FlexfiError::Unauthorized.into());
    }
    
    Ok(())
}

// Initialiser la whitelist (appelé une seule fois par un admin)
pub fn process_initialize_whitelist(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let whitelist_account = next_account_info(account_info_iter)?;
    let authority = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    
    // Vérifier que l'autorité est le signataire
    if !authority.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Créer le PDA pour la whitelist
    let (whitelist_pda, bump) = Pubkey::find_program_address(
        &[WHITELIST_SEED],
        program_id
    );
    
    if whitelist_account.key != &whitelist_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Créer le compte
    let rent = Rent::get()?;
    let space = WhitelistAccount::SIZE;
    let rent_lamports = rent.minimum_balance(space);
    
    invoke_signed(
        &system_instruction::create_account(
            authority.key,
            &whitelist_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[authority.clone(), whitelist_account.clone(), system_program.clone()],
        &[&[WHITELIST_SEED, &[bump]]],
    )?;
    
    // Initialiser les données
    let whitelist_data = WhitelistAccount {
        authority: *authority.key,
        is_active: true,
        total_users: 0,
        bump,
    };
    
    whitelist_data.serialize(&mut *whitelist_account.data.borrow_mut())?;
    
    msg!("Whitelist initialized with authority: {}", authority.key);
    Ok(())
}

// Ajouter un utilisateur à la whitelist (appelé par le backend)
pub fn process_add_to_whitelist(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    user_pubkey: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let whitelist_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let authority = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier l'autorité
    if !authority.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Charger la whitelist
    let mut whitelist_data = WhitelistAccount::try_from_slice(&whitelist_account.data.borrow())?;
    
    // Vérifier que l'autorité est correcte
    if whitelist_data.authority != *authority.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Créer le PDA pour le statut de l'utilisateur
    let (user_status_pda, user_bump) = Pubkey::find_program_address(
        &[WHITELIST_SEED, user_pubkey.as_ref()],
        program_id
    );
    
    if user_status_account.key != &user_status_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Obtenir l'horodatage
    let clock = Clock::from_account_info(clock_sysvar)?;
    
    // Créer le compte de statut utilisateur
    let rent = Rent::get()?;
    let space = UserWhitelistStatus::SIZE;
    let rent_lamports = rent.minimum_balance(space);
    
    invoke_signed(
        &system_instruction::create_account(
            authority.key,
            &user_status_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[authority.clone(), user_status_account.clone(), system_program.clone()],
        &[&[WHITELIST_SEED, user_pubkey.as_ref(), &[user_bump]]],
    )?;
    
    // Initialiser le statut
    let user_status = UserWhitelistStatus {
        user_pubkey,
        is_whitelisted: true,
        whitelisted_at: clock.unix_timestamp,
        whitelisted_by: *authority.key,
        bump: user_bump,
    };
    
    user_status.serialize(&mut *user_status_account.data.borrow_mut())?;
    
    // Mettre à jour le compteur
    whitelist_data.total_users += 1;
    whitelist_data.serialize(&mut *whitelist_account.data.borrow_mut())?;
    
    msg!("User {} added to whitelist", user_pubkey);
    Ok(())
}

pub fn process_remove_from_whitelist(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    user_pubkey: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let whitelist_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let authority = next_account_info(account_info_iter)?;
    
    // Vérifier l'autorité
    if !authority.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Charger la whitelist
    let mut whitelist_data = WhitelistAccount::try_from_slice(&whitelist_account.data.borrow())?;
    
    // Vérifier que l'autorité est correcte
    if whitelist_data.authority != *authority.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier le PDA du statut utilisateur
    let (user_status_pda, _) = Pubkey::find_program_address(
        &[WHITELIST_SEED, user_pubkey.as_ref()],
        program_id
    );
    
    if user_status_account.key != &user_status_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Charger le statut de l'utilisateur
    let mut user_status = UserWhitelistStatus::try_from_slice(&user_status_account.data.borrow())?;
    
    // Vérifier que c'est le bon utilisateur
    if user_status.user_pubkey != user_pubkey {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Marquer comme non whitelisté
    user_status.is_whitelisted = false;
    user_status.serialize(&mut *user_status_account.data.borrow_mut())?;
    
    // Décrémenter le compteur (attention aux underflows)
    whitelist_data.total_users = whitelist_data.total_users.saturating_sub(1);
    whitelist_data.serialize(&mut *whitelist_account.data.borrow_mut())?;
    
    msg!("User {} removed from whitelist", user_pubkey);
    Ok(())
}