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
use crate::core::whitelist::require_whitelisted;
use crate::error::FlexfiError;
use crate::state::nft::{NFTMetadataAccount, NFTAttachmentAccount};
use crate::constants::{NFT_METADATA_SEED, NFT_ATTACHMENT_SEED};

pub fn process_attach_nft(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    card_id: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let attachment_account = next_account_info(account_info_iter)?;
    let nft_metadata_account = next_account_info(account_info_iter)?;
    let nft_mint = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
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

    // Vérifier les métadonnées NFT
    let nft_seeds = [
        NFT_METADATA_SEED,
        nft_mint.key.as_ref(),
    ];
    let (nft_metadata_pda, _) = Pubkey::find_program_address(&nft_seeds, program_id);
    
    if *nft_metadata_account.key != nft_metadata_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Charger les métadonnées NFT
    let nft_metadata = NFTMetadataAccount::try_from_slice(&nft_metadata_account.data.borrow())?;
    
    // Vérifier la propriété du NFT
    if nft_metadata.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Vérifier si le NFT est actif
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    
    if !nft_metadata.is_active || nft_metadata.is_expired(current_time) {
        return Err(FlexfiError::NFTExpired.into());
    }
    
    // Créer un PDA pour l'attachement
    let attachment_seeds = [
        NFT_ATTACHMENT_SEED,
        nft_mint.key.as_ref(),
        &card_id,
    ];
    let (attachment_pda, attachment_bump) = Pubkey::find_program_address(&attachment_seeds, program_id);
    
    if *attachment_account.key != attachment_pda {
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Créer le compte d'attachement
    let rent = Rent::get()?;
    let space = NFTAttachmentAccount::SIZE;
    let rent_lamports = rent.minimum_balance(space);
    
    invoke_signed(
        &system_instruction::create_account(
            user_account.key,
            &attachment_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[user_account.clone(), attachment_account.clone(), system_program.clone()],
        &[&[NFT_ATTACHMENT_SEED, nft_mint.key.as_ref(), &card_id, &[attachment_bump]]],
    )?;
    
    // Initialiser les données d'attachement
    let attachment = NFTAttachmentAccount::new(
        *nft_mint.key,
        *user_account.key,
        card_id,
        current_time,
        attachment_bump,
    );
    
    attachment.serialize(&mut *attachment_account.data.borrow_mut())?;
    
    msg!("NFT attached to card successfully");
    Ok(())
}

pub fn process_detach_nft(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    let attachment_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;
    
    // Vérifier la signature de l'utilisateur
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Charger les données d'attachement
    let mut attachment = NFTAttachmentAccount::try_from_slice(&attachment_account.data.borrow())?;
    
    // Vérifier la propriété
    if attachment.user_wallet != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }
    
    // Désactiver l'attachement
    attachment.is_active = false;
    
    // Mettre à jour l'horodatage
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;
    attachment.attached_at = current_time; // Utiliser attached_at comme "detached_at"
    
    // Sauvegarder les modifications
    attachment.serialize(&mut *attachment_account.data.borrow_mut())?;
    
    msg!("NFT detached from card");
    Ok(())
}

pub struct NFTAttacher;

impl NFTAttacher {
    pub fn attach_nft(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        card_id: [u8; 32],
    ) -> ProgramResult {
        process_attach_nft(program_id, accounts, card_id)
    }
}