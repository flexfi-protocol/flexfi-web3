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
use crate::state::nft::{NFTMetadataAccount, NFTType};
use crate::constants::{NFT_METADATA_SEED, NFT_MINT_COST, NFT_NONE, NFT_BRONZE, NFT_SILVER, NFT_GOLD};

pub fn process_mint_nft(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    nft_type: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let metadata_account = next_account_info(account_info_iter)?;
    let mint_account = next_account_info(account_info_iter)?;
    let mint_authority = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let fee_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check signatures
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;

    if !mint_authority.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Check if the NFT type is valid
    if nft_type < NFT_BRONZE || nft_type > NFT_GOLD {
        return Err(FlexfiError::InvalidNFTType.into());
    }

    // Create a PDA for NFT metadata
    let seeds = [
        NFT_METADATA_SEED,
        mint_account.key.as_ref(),
    ];
    let (metadata_pda, metadata_bump) = Pubkey::find_program_address(&seeds, program_id);

    if *metadata_account.key != metadata_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create the metadata account
    let rent = Rent::get()?;
    let space = NFTMetadataAccount::SIZE;
    let rent_lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            user_account.key,
            &metadata_pda,
            rent_lamports,
            space as u64,
            program_id,
        ),
        &[user_account.clone(), metadata_account.clone(), system_program.clone()],
        &[&[NFT_METADATA_SEED, mint_account.key.as_ref(), &[metadata_bump]]],
    )?;

    // Determine the NFT validity duration (default 1 year)
    let duration_days = 365u16;

    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Initialize metadata
    let metadata = NFTMetadataAccount::new(
        *mint_account.key,
        *user_account.key,
        NFTType::from_u8(nft_type)?,
        1, // Default level 1
        duration_days,
        current_time,
        metadata_bump,
    );

    metadata.serialize(&mut *metadata_account.data.borrow_mut())?;

    // Mint an NFT token for the user
    let mint_to_ix = spl_token::instruction::mint_to(
        token_program.key,
        mint_account.key,
        user_token_account.key,
        mint_authority.key,
        &[],
        1, // Mint 1 token (NFT)
    )?;

    invoke(
        &mint_to_ix,
        &[
            mint_account.clone(),
            user_token_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
        ],
    )?;

    // Transfer mint fees
    let transfer_fee_ix = spl_token::instruction::transfer(
        token_program.key,
        user_token_account.key,
        fee_account.key,
        user_account.key,
        &[],
        NFT_MINT_COST,
    )?;

    invoke(
        &transfer_fee_ix,
        &[
            user_token_account.clone(),
            fee_account.clone(),
            user_account.clone(),
            token_program.clone(),
        ],
    )?;

    let nft_type_name = match nft_type {
        NFT_BRONZE => "Bronze",
        NFT_SILVER => "Silver",
        NFT_GOLD => "Gold",
        _ => "Unknown",
    };

    msg!("NFT minted successfully: type={}, level={}, duration={} days",
         nft_type_name, 1, duration_days);
    Ok(())
}

pub fn process_is_nft_active(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let metadata_account = next_account_info(account_info_iter)?;
    let mint_account = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Verify the metadata account
    let seeds = [
        NFT_METADATA_SEED,
        mint_account.key.as_ref(),
    ];
    let (metadata_pda, _) = Pubkey::find_program_address(&seeds, program_id);

    if *metadata_account.key != metadata_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Load metadata
    let metadata = NFTMetadataAccount::try_from_slice(&metadata_account.data.borrow())?;

    // Check if the NFT is active and not expired
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    let is_active = metadata.is_active && !metadata.is_expired(current_time);

    msg!("NFT is active: {}", is_active);
    Ok(())
}

pub fn process_extend_nft_duration(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    additional_days: u16,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let metadata_account = next_account_info(account_info_iter)?;
    let mint_account = next_account_info(account_info_iter)?;
    let owner_account = next_account_info(account_info_iter)?;
    let fee_account = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let _clock_sysvar = next_account_info(account_info_iter)?;

    // Check owner signature
    if !owner_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Verify the metadata account
    let seeds = [
        NFT_METADATA_SEED,
        mint_account.key.as_ref(),
    ];
    let (metadata_pda, _) = Pubkey::find_program_address(&seeds, program_id);

    if *metadata_account.key != metadata_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Load metadata
    let mut metadata = NFTMetadataAccount::try_from_slice(&metadata_account.data.borrow())?;

    // Verify ownership
    if metadata.owner != *owner_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Calculate the cost of extension (e.g., 1 USDC per day)
    let extension_cost = (additional_days as u64).saturating_mul(1_000_000); // 1 USDC per day

    // Transfer extension fees
    let transfer_fee_ix = spl_token::instruction::transfer(
        token_program.key,
        user_token_account.key,
        fee_account.key,
        owner_account.key,
        &[],
        extension_cost,
    )?;

    invoke(
        &transfer_fee_ix,
        &[
            user_token_account.clone(),
            fee_account.clone(),
            owner_account.clone(),
            token_program.clone(),
        ],
    )?;

    // Update the NFT duration
    metadata.extend_duration(additional_days);

    // Reactivate the NFT if it was inactive
    metadata.is_active = true;

    // Save changes
    metadata.serialize(&mut *metadata_account.data.borrow_mut())?;

    msg!("NFT duration extended by {} days, new expiry: {}",
         additional_days, metadata.expiry_time);
    Ok(())
}

pub struct NFTMinter;

impl NFTMinter {
    pub fn mint_nft(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        nft_type: u8,
    ) -> ProgramResult {
        process_mint_nft(program_id, accounts, nft_type)
    }
}
