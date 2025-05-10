use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct WalletAccount {
    pub owner: Pubkey,
    pub is_active: bool,
    pub card_type: u8,
    pub created_at: i64,
    pub bump: u8,
}

impl WalletAccount {
    pub const SIZE: usize = 32 + 1 + 1 + 8 + 1; // 43 bytes
}