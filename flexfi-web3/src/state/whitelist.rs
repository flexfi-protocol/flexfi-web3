use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct WhitelistAccount {
    pub authority: Pubkey,
    pub is_active: bool,
    pub total_users: u64,
    pub bump: u8,
}

impl WhitelistAccount {
    pub const SIZE: usize = 32 + 1 + 8 + 1; // 42 bytes
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct UserWhitelistStatus {
    pub user_pubkey: Pubkey,
    pub is_whitelisted: bool,
    pub whitelisted_at: i64,
    pub whitelisted_by: Pubkey,
    pub bump: u8,
}

impl UserWhitelistStatus {
    pub const SIZE: usize = 32 + 1 + 8 + 32 + 1; // 74 bytes
}