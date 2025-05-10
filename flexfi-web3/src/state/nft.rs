use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone, Copy)]
pub enum NFTType {
    None,
    Bronze,
    Silver,
    Gold,
}

impl NFTType {
    pub fn to_u8(&self) -> u8 {
        match self {
            NFTType::None => 0,
            NFTType::Bronze => 1,
            NFTType::Silver => 2,
            NFTType::Gold => 3,
        }
    }
    
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(NFTType::None),
            1 => Ok(NFTType::Bronze),
            2 => Ok(NFTType::Silver),
            3 => Ok(NFTType::Gold),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct NFTMetadataAccount {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub nft_type: u8,
    pub level: u8,
    pub duration_days: u16,
    pub creation_time: i64,
    pub expiry_time: i64,
    pub is_active: bool,
    pub bump: u8,
}

impl NFTMetadataAccount {
    pub const SIZE: usize = 32 + 32 + 1 + 1 + 2 + 8 + 8 + 1 + 1; // 86 bytes
    
    pub fn new(
        mint: Pubkey,
        owner: Pubkey,
        nft_type: NFTType,
        level: u8,
        duration_days: u16,
        creation_time: i64,
        bump: u8,
    ) -> Self {
        let expiry_time = creation_time + (duration_days as i64 * 86400);
        
        Self {
            mint,
            owner,
            nft_type: nft_type.to_u8(),
            level,
            duration_days,
            creation_time,
            expiry_time,
            is_active: true,
            bump,
        }
    }
    
    pub fn get_nft_type(&self) -> Result<NFTType, ProgramError> {
        NFTType::from_u8(self.nft_type)
    }
    
    pub fn is_expired(&self, current_time: i64) -> bool {
        current_time >= self.expiry_time
    }
    
    pub fn extend_duration(&mut self, additional_days: u16) {
        self.duration_days = self.duration_days.saturating_add(additional_days);
        self.expiry_time = self.expiry_time.saturating_add((additional_days as i64) * 86400);
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct NFTAttachmentAccount {
    pub nft_mint: Pubkey,
    pub user_wallet: Pubkey,
    pub card_id: [u8; 32],
    pub attached_at: i64,
    pub is_active: bool,
    pub bump: u8,
}

impl NFTAttachmentAccount {
    pub const SIZE: usize = 32 + 32 + 32 + 8 + 1 + 1; // 106 bytes
    
    pub fn new(
        nft_mint: Pubkey,
        user_wallet: Pubkey,
        card_id: [u8; 32],
        attached_at: i64,
        bump: u8,
    ) -> Self {
        Self {
            nft_mint,
            user_wallet,
            card_id,
            attached_at,
            is_active: true,
            bump,
        }
    }
}