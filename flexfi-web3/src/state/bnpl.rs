use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone, Copy)]
pub enum BNPLStatus {
    Active,
    Completed,
    Defaulted,
    Cancelled,
}

impl BNPLStatus {
    pub fn to_u8(&self) -> u8 {
        match self {
            BNPLStatus::Active => 0,
            BNPLStatus::Completed => 1,
            BNPLStatus::Defaulted => 2,
            BNPLStatus::Cancelled => 3,
        }
    }

    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(BNPLStatus::Active),
            1 => Ok(BNPLStatus::Completed),
            2 => Ok(BNPLStatus::Defaulted),
            3 => Ok(BNPLStatus::Cancelled),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BNPLContractAccount {
    pub borrower: Pubkey,
    pub merchant: Pubkey,
    pub amount: u64,
    pub token_mint: Pubkey,
    pub installments: u8,
    pub paid_installments: u8,
    pub next_payment_due: i64,
    pub payment_interval_days: u8,
    pub amount_per_installment: u64,
    pub status: u8,
    pub created_at: i64,
    pub last_payment_at: i64,
    pub fee_percentage: u16,
    pub apr_percentage: u16,
    pub card_type: u8,
    pub nft_type: u8,
    pub bump: u8,
}

impl BNPLContractAccount {
    pub const SIZE: usize = 32 + 32 + 8 + 32 + 1 + 1 + 8 + 1 + 8 + 1 + 8 + 8 + 2 + 2 + 1 + 1 + 1; // 147 bytes

    pub fn new(
        borrower: Pubkey,
        merchant: Pubkey,
        amount: u64,
        token_mint: Pubkey,
        installments: u8,
        payment_interval_days: u8,
        amount_per_installment: u64,
        fee_percentage: u16,
        apr_percentage: u16,
        card_type: u8,
        nft_type: u8,
        created_at: i64,
        next_payment_due: i64,
        bump: u8,
    ) -> Self {
        Self {
            borrower,
            merchant,
            amount,
            token_mint,
            installments,
            paid_installments: 0,
            next_payment_due,
            payment_interval_days,
            amount_per_installment,
            status: BNPLStatus::Active.to_u8(),
            created_at,
            last_payment_at: created_at,
            fee_percentage,
            apr_percentage,
            card_type,
            nft_type,
            bump,
        }
    }

    pub fn get_status(&self) -> Result<BNPLStatus, ProgramError> {
        BNPLStatus::from_u8(self.status)
    }

    pub fn set_status(&mut self, status: BNPLStatus) {
        self.status = status.to_u8();
    }

    pub fn is_payment_due(&self, current_time: i64) -> bool {
        current_time >= self.next_payment_due
    }

    pub fn update_after_payment(&mut self, current_time: i64) -> Result<(), ProgramError> {
        self.paid_installments += 1;
        self.last_payment_at = current_time;

        if self.paid_installments >= self.installments {
            self.set_status(BNPLStatus::Completed);
        } else {
            // Calculate the next due date
            self.next_payment_due = current_time + (self.payment_interval_days as i64 * 86400);
        }

        Ok(())
    }

    pub fn remaining_amount(&self) -> u64 {
        let remaining_installments = self.installments.saturating_sub(self.paid_installments);
        self.amount_per_installment.saturating_mul(remaining_installments as u64)
    }
}
