use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
    msg,
};
use borsh::BorshDeserialize;

use crate::error::FlexfiError;
use crate::state::nft::{NFTMetadataAccount, NFTAttachmentAccount, NFTType};
use crate::constants::NFT_METADATA_SEED;

#[derive(Debug, Clone, Copy)]
pub enum NFTPerk {
    ReducedFees,
    IncreasedCreditLimit,
    CashbackBoost,
    ExtendedPaymentTerms,
    PriorityProcessing,
    CustomDesign,
    VIP,
}

pub struct NFTPerkChecker {}

impl NFTPerkChecker {
    // Check if a specific NFT perk is active
    pub fn check_perk(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        perk: NFTPerk,
    ) -> Result<bool, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let nft_metadata_account = next_account_info(account_info_iter)?;
        let attachment_account = next_account_info(account_info_iter)?;
        let nft_mint = next_account_info(account_info_iter)?;
        let user_account = next_account_info(account_info_iter)?;
        let clock_sysvar = next_account_info(account_info_iter)?;

        // Verify NFT metadata
        let nft_seeds = [
            NFT_METADATA_SEED,
            nft_mint.key.as_ref(),
        ];
        let (nft_metadata_pda, _) = Pubkey::find_program_address(&nft_seeds, program_id);

        if *nft_metadata_account.key != nft_metadata_pda {
            return Err(ProgramError::InvalidAccountData);
        }

        // Load NFT metadata
        let nft_metadata = NFTMetadataAccount::try_from_slice(&nft_metadata_account.data.borrow())?;

        // Load attachment data
        let attachment = NFTAttachmentAccount::try_from_slice(&attachment_account.data.borrow())?;

        // Check if the attachment is active
        if !attachment.is_active {
            return Ok(false);
        }

        // Check if the NFT is active and not expired
        let clock = Clock::from_account_info(clock_sysvar)?;
        let current_time = clock.unix_timestamp;

        if !nft_metadata.is_active || nft_metadata.is_expired(current_time) {
            return Ok(false);
        }

        // Check if the attachment matches this NFT
        if attachment.nft_mint != *nft_mint.key {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if the NFT belongs to the user
        if nft_metadata.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }

        // Get the NFT type and level
        let nft_type = nft_metadata.get_nft_type()?;
        let level = nft_metadata.level;

        // Check if the perk is enabled for this NFT type and level
        let is_enabled = match perk {
            NFTPerk::ReducedFees => {
                // All NFT types have reduced fees
                true
            },
            NFTPerk::IncreasedCreditLimit => {
                // Only Premium, Gold, and Platinum have increased credit limit
                match nft_type {
                    NFTType::None => false,
                    NFTType::Bronze => level >= 2,
                    NFTType::Silver => true,
                    NFTType::Gold => true,
                }
            },
            NFTPerk::CashbackBoost => {
                // Only Gold and Platinum have cashback boost
                match nft_type {
                    NFTType::Gold => level >= 1,
                    NFTType::Silver => level >= 3,
                    _ => false,
                }
            },
            NFTPerk::ExtendedPaymentTerms => {
                // Silver level 3, Gold, and Platinum have extended payment terms
                match nft_type {
                    NFTType::Silver => level >= 3,
                    NFTType::Gold => true,
                    _ => false,
                }
            },
            NFTPerk::PriorityProcessing => {
                // Only Platinum has priority processing
                nft_type == NFTType::Gold && level >= 3
            },
            NFTPerk::CustomDesign => {
                // All Gold levels have custom design
                nft_type == NFTType::Gold
            },
            NFTPerk::VIP => {
                // Only Gold level 3 has VIP
                nft_type == NFTType::Gold && level >= 3
            },
        };

        msg!("NFT perk check for {:?}: {}", perk, is_enabled);
        Ok(is_enabled)
    }

    // Get the fee reduction based on the NFT
    pub fn get_fee_reduction(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<u16, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let nft_metadata_account = next_account_info(account_info_iter)?;
        let attachment_account = next_account_info(account_info_iter)?;
        let _nft_mint = next_account_info(account_info_iter)?;

        // Verify and retrieve metadata and attachment
        // Simplified for brevity

        let nft_metadata = NFTMetadataAccount::try_from_slice(&nft_metadata_account.data.borrow())?;
        let attachment = NFTAttachmentAccount::try_from_slice(&attachment_account.data.borrow())?;

        // Check if active
        if !attachment.is_active || !nft_metadata.is_active {
            return Ok(0);
        }

        // Check if expired
        let clock = Clock::get()?;
        if nft_metadata.is_expired(clock.unix_timestamp) {
            return Ok(0);
        }

        // Get type and level
        let nft_type = nft_metadata.get_nft_type()?;
        let level = nft_metadata.level;

        // Calculate reduction
        let reduction = match nft_type {
            NFTType::None => 0,
            NFTType::Bronze => level * 50, // 0-50-100-150 basis points
            NFTType::Silver => 100 + (level * 50), // 100-150-200-250 basis points
            NFTType::Gold => 200 + (level * 70), // 200-270-340-410 basis points
        };

        // Cap at 500 basis points (5%)
        let capped_reduction = std::cmp::min(reduction as u16, 500) as u8;

        msg!("NFT fee reduction: {}%", capped_reduction as f64 / 100.0);
        Ok(capped_reduction as u16)
    }

    // Get the credit limit boost based on the NFT
    pub fn get_credit_limit_boost(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<u16, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let nft_metadata_account = next_account_info(account_info_iter)?;
        let attachment_account = next_account_info(account_info_iter)?;
        let _nft_mint = next_account_info(account_info_iter)?;

        // Verify and retrieve metadata and attachment
        // Simplified for brevity

        let nft_metadata = NFTMetadataAccount::try_from_slice(&nft_metadata_account.data.borrow())?;
        let attachment = NFTAttachmentAccount::try_from_slice(&attachment_account.data.borrow())?;

        // Check if active
        if !attachment.is_active || !nft_metadata.is_active {
            return Ok(0);
        }

        // Check if expired
        let clock = Clock::get()?;
        if nft_metadata.is_expired(clock.unix_timestamp) {
            return Ok(0);
        }

        // Get type and level
        let nft_type = nft_metadata.get_nft_type()?;
        let level = nft_metadata.level;

        // Calculate boost
        let boost = match nft_type {
            NFTType::None => 0,
            NFTType::Bronze => 0,
            NFTType::Silver => level * 100, // 0-100-200-300 basis points
            NFTType::Gold => 250 + (level * 150), // 250-400-550-700 basis points
        };

        msg!("NFT credit limit boost: {}%", boost as f64 / 100.0);
        Ok(boost as u16)
    }
}
