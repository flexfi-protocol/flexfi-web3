use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AuthorizationAccount {
    pub user: Pubkey,
    pub flexfi_authority: Pubkey,
    pub authorized_amount: u64,
    pub used_amount: u64,
    pub is_active: bool,
    pub created_at: i64,
    pub expires_at: i64,
    pub bump: u8,
}

impl AuthorizationAccount {
    pub const SIZE: usize = 32 + 32 + 8 + 8 + 1 + 8 + 8 + 1; // 98 bytes
    
    pub fn remaining_credit(&self) -> u64 {
        self.authorized_amount.saturating_sub(self.used_amount)
    }
    
    pub fn is_valid(&self, current_time: i64) -> bool {
        self.is_active && current_time < self.expires_at
    }
}