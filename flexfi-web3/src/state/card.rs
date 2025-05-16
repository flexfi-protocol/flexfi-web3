use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CardAccount {
    pub owner: Pubkey,
    pub card_type: u8,
    pub issued_at: i64,
    pub expires_at: i64,
    pub is_active: bool,
    pub annual_fee_paid_until: i64,
    pub bump: u8,
}

impl CardAccount {
    pub const SIZE: usize = 32 + 1 + 8 + 8 + 1 + 8 + 1; // 59 bytes

    pub fn new(
        owner: Pubkey,
        card_type: u8,
        issued_at: i64,
        bump: u8,
    ) -> Self {
        // Card valid for 3 years
        let expires_at = issued_at + (3 * 365 * 86400);

        Self {
            owner,
            card_type,
            issued_at,
            expires_at,
            is_active: true,
            annual_fee_paid_until: issued_at + (365 * 86400), // Paid for 1 year
            bump,
        }
    }

    pub fn is_expired(&self, current_time: i64) -> bool {
        current_time >= self.expires_at
    }

    pub fn is_fee_due(&self, current_time: i64) -> bool {
        current_time >= self.annual_fee_paid_until
    }

    pub fn pay_annual_fee(&mut self, current_time: i64) {
        // Add 1 year to the fee expiration date
        self.annual_fee_paid_until = current_time + (365 * 86400);
    }
}
