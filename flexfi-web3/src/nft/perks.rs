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
    // Vérifier si un avantage NFT spécifique est actif
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
        
        // Charger les données d'attachement
        let attachment = NFTAttachmentAccount::try_from_slice(&attachment_account.data.borrow())?;
        
        // Vérifier si l'attachement est actif
        if !attachment.is_active {
            return Ok(false);
        }
        
        // Vérifier si le NFT est actif et non expiré
        let clock = Clock::from_account_info(clock_sysvar)?;
        let current_time = clock.unix_timestamp;
        
        if !nft_metadata.is_active || nft_metadata.is_expired(current_time) {
            return Ok(false);
        }
        
        // Vérifier si l'attachement correspond à ce NFT
        if attachment.nft_mint != *nft_mint.key {
            return Err(ProgramError::InvalidAccountData);
        }
        
        // Vérifier si le NFT appartient à l'utilisateur
        if nft_metadata.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }
        
        // Récupérer le type et le niveau du NFT
        let nft_type = nft_metadata.get_nft_type()?;
        let level = nft_metadata.level;
        
        // Vérifier si l'avantage est activé pour ce type et niveau de NFT
        let is_enabled = match perk {
            NFTPerk::ReducedFees => {
                // Tous les types de NFT ont des frais réduits
                true
            },
            NFTPerk::IncreasedCreditLimit => {
                // Seulement Premium, Gold, et Platinum ont une limite de crédit augmentée
                match nft_type {
                    NFTType::None => false,
                    NFTType::Bronze => level >= 2,
                    NFTType::Silver => true,
                    NFTType::Gold => true,
                }
            },
            NFTPerk::CashbackBoost => {
                // Seulement Gold et Platinum ont un boost de cashback
                match nft_type {
                    NFTType::Gold => level >= 1,
                    NFTType::Silver => level >= 3,
                    _ => false,
                }
            },
            NFTPerk::ExtendedPaymentTerms => {
                // Silver niveau 3, Gold, et Platinum ont des conditions de paiement étendues
                match nft_type {
                    NFTType::Silver => level >= 3,
                    NFTType::Gold => true,
                    _ => false,
                }
            },
            NFTPerk::PriorityProcessing => {
                // Seulement Platinum a un traitement prioritaire
                nft_type == NFTType::Gold && level >= 3
            },
            NFTPerk::CustomDesign => {
                // Tous les niveaux de Gold ont un design personnalisé
                nft_type == NFTType::Gold
            },
            NFTPerk::VIP => {
                // Seulement Gold niveau 3 a VIP
                nft_type == NFTType::Gold && level >= 3
            },
        };
        
        msg!("NFT perk check for {:?}: {}", perk, is_enabled);
        Ok(is_enabled)
    }
    
    // Obtenir la réduction de frais en fonction du NFT
    pub fn get_fee_reduction(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<u16, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        
        let nft_metadata_account = next_account_info(account_info_iter)?;
        let attachment_account = next_account_info(account_info_iter)?;
        let _nft_mint = next_account_info(account_info_iter)?;
        
        // Vérifier et récupérer les métadonnées et l'attachement
        // Simplifié pour la brièveté
        
        let nft_metadata = NFTMetadataAccount::try_from_slice(&nft_metadata_account.data.borrow())?;
        let attachment = NFTAttachmentAccount::try_from_slice(&attachment_account.data.borrow())?;
        
        // Vérifier si actif
        if !attachment.is_active || !nft_metadata.is_active {
            return Ok(0);
        }
        
        // Vérifier si expiré
        let clock = Clock::get()?;
        if nft_metadata.is_expired(clock.unix_timestamp) {
            return Ok(0);
        }
        
        // Récupérer le type et le niveau
        let nft_type = nft_metadata.get_nft_type()?;
        let level = nft_metadata.level;
        
        // Calculer la réduction
        let reduction = match nft_type {
            NFTType::None => 0,
            NFTType::Bronze => level * 50, // 0-50-100-150 points de base
            NFTType::Silver => 100 + (level * 50), // 100-150-200-250 points de base
            NFTType::Gold => 200 + (level * 70), // 200-270-340-410 points de base
        };
        
        // Plafonner à 500 points de base (5%)
        let capped_reduction = std::cmp::min(reduction as u16, 500) as u8;
        
        msg!("NFT fee reduction: {}%", capped_reduction as f64 / 100.0);
        Ok(capped_reduction as u16)
    }
    
    // Obtenir l'augmentation de limite de crédit en fonction du NFT
    pub fn get_credit_limit_boost(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<u16, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        
        let nft_metadata_account = next_account_info(account_info_iter)?;
        let attachment_account = next_account_info(account_info_iter)?;
        let _nft_mint = next_account_info(account_info_iter)?;
        
        // Vérifier et récupérer les métadonnées et l'attachement
        // Simplifié pour la brièveté
        
        let nft_metadata = NFTMetadataAccount::try_from_slice(&nft_metadata_account.data.borrow())?;
        let attachment = NFTAttachmentAccount::try_from_slice(&attachment_account.data.borrow())?;
        
        // Vérifier si actif
        if !attachment.is_active || !nft_metadata.is_active {
            return Ok(0);
        }
        
        // Vérifier si expiré
        let clock = Clock::get()?;
        if nft_metadata.is_expired(clock.unix_timestamp) {
            return Ok(0);
        }
        
        // Récupérer le type et le niveau
        let nft_type = nft_metadata.get_nft_type()?;
        let level = nft_metadata.level;
        
        // Calculer l'augmentation
        let boost = match nft_type {
            NFTType::None => 0,
            NFTType::Bronze => 0,
            NFTType::Silver => level * 100, // 0-100-200-300 points de base
            NFTType::Gold => 250 + (level * 150), // 250-400-550-700 points de base
        };
        
        msg!("NFT credit limit boost: {}%", boost as f64 / 100.0);
        Ok(boost as u16)
    }
}