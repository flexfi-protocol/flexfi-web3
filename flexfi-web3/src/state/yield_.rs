use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone, Copy)]
pub enum YieldStrategy {
    AutoCompound,
    StableCoin,
    HighYield,
    RealWorldAssets,
    Custom,
}

impl YieldStrategy {
    pub fn to_u8(&self) -> u8 {
        match self {
            YieldStrategy::AutoCompound => 0,
            YieldStrategy::StableCoin => 1,
            YieldStrategy::HighYield => 2,
            YieldStrategy::RealWorldAssets => 3,
            YieldStrategy::Custom => 4,
        }
    }
    
    pub fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            0 => Ok(YieldStrategy::AutoCompound),
            1 => Ok(YieldStrategy::StableCoin),
            2 => Ok(YieldStrategy::HighYield),
            3 => Ok(YieldStrategy::RealWorldAssets),
            4 => Ok(YieldStrategy::Custom),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct YieldAccount {
    pub owner: Pubkey,
    pub strategy: u8,
    pub custom_strategy_address: Pubkey,
    pub auto_reinvest: bool,
    pub total_yield_earned: u64,
    pub total_yield_claimed: u64,
    pub last_yield_claimed: i64,
    pub created_at: i64,
    pub bump: u8,
}

impl YieldAccount {
    pub const SIZE: usize = 32 + 1 + 32 + 1 + 8 + 8 + 8 + 8 + 1; // 99 bytes
    
    pub fn new(
        owner: Pubkey,
        strategy: YieldStrategy,
        custom_strategy_address: Pubkey,
        auto_reinvest: bool,
        created_at: i64,
        bump: u8,
    ) -> Self {
        Self {
            owner,
            strategy: strategy.to_u8(),
            custom_strategy_address,
            auto_reinvest,
            total_yield_earned: 0,
            total_yield_claimed: 0,
            last_yield_claimed: created_at,
            created_at,
            bump,
        }
    }
    
    pub fn get_strategy(&self) -> Result<YieldStrategy, ProgramError> {
        YieldStrategy::from_u8(self.strategy)
    }
    
    pub fn set_strategy(&mut self, strategy: YieldStrategy) {
        self.strategy = strategy.to_u8();
    }
    
    pub fn record_yield_earned(&mut self, amount: u64) {
        self.total_yield_earned = self.total_yield_earned.saturating_add(amount);
    }
    
    pub fn record_yield_claimed(&mut self, amount: u64, current_time: i64) -> Result<(), ProgramError> {
        if amount > self.get_unclaimed_yield() {
            return Err(ProgramError::InsufficientFunds);
        }
        
        self.total_yield_claimed = self.total_yield_claimed.saturating_add(amount);
        self.last_yield_claimed = current_time;
        
        Ok(())
    }
    
    pub fn get_unclaimed_yield(&self) -> u64 {
        self.total_yield_earned.saturating_sub(self.total_yield_claimed)
    }
}