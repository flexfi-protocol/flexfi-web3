use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
    account_info::AccountInfo,
    entrypoint::ProgramResult,
};
use crate::core::staking::process_deposit_staking;


#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone, Copy)]
pub enum StakingStatus {
    Active,
    Locked,
    Frozen,
    Closed,
}

impl StakingStatus {
    pub fn to_u8(&self) -> u8 {
        match self {
            StakingStatus::Active => 0,
            StakingStatus::Locked => 1,
            StakingStatus::Frozen => 2,
            StakingStatus::Closed => 3,
        }
    }
    
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(StakingStatus::Active),
            1 => Ok(StakingStatus::Locked),
            2 => Ok(StakingStatus::Frozen),
            3 => Ok(StakingStatus::Closed),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct StakingAccount {
    pub owner: Pubkey,
    pub usdc_mint: Pubkey,
    pub amount_staked: u64,
    pub status: u8,
    pub lock_period_end: i64,
    pub created_at: i64,
    pub last_update: i64,
    pub bump: u8,
}

impl StakingAccount {
    pub const SIZE: usize = 32 + 32 + 8 + 1 + 8 + 8 + 8 + 1; // 98 bytes
    
    pub fn new(
        owner: Pubkey,
        usdc_mint: Pubkey,
        amount_staked: u64,
        status: StakingStatus,
        lock_period_end: i64,
        created_at: i64,
        bump: u8,
    ) -> Self {
        Self {
            owner,
            usdc_mint,
            amount_staked,
            status: status.to_u8(),
            lock_period_end,
            created_at,
            last_update: created_at,
            bump,
        }
    }
    
    pub fn get_status(&self) -> Result<StakingStatus, ProgramError> {
        StakingStatus::from_u8(self.status)
    }
    
    pub fn set_status(&mut self, status: StakingStatus) {
        self.status = status.to_u8();
    }
}

pub struct StakingManager;

impl StakingManager {
    pub fn deposit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
        lock_days: u16,
    ) -> ProgramResult {
        process_deposit_staking(program_id, accounts, amount, lock_days)
    }
}