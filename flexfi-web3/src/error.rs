use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum FlexfiError {
    #[error("Protocol is paused")]
    ProtocolPaused,

    #[error("Amount exceeds the maximum allowed")]
    AmountTooHigh,

    #[error("Too many loans this year")]
    TooManyLoans,

    #[error("Insufficient collateral")]
    InsufficientCollateral,

    #[error("Arithmetic overflow")]
    MathOverflow,

    #[error("Invalid number of installments")]
    InvalidInstallments,

    #[error("Installment not allowed for this card type")]
    InvalidInstallmentForCard,

    #[error("Loan is inactive")]
    LoanNotActive,

    #[error("Loan already repaid")]
    LoanAlreadyPaid,

    #[error("Grace period not expired")]
    GracePeriodNotExpired,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Fees too high")]
    FeeTooHigh,

    #[error("Already at this level")]
    AlreadyAtThisLevel,

    #[error("Invalid card type")]
    InvalidCardType,

    #[error("Invalid NFT type")]
    InvalidNFTType,

    #[error("No yield to claim")]
    NoYieldToClaim,

    #[error("Insufficient staking")]
    InsufficientStaking,

    #[error("Staking not active")]
    StakingNotActive,

    #[error("Staking frozen")]
    StakingFrozen,

    #[error("Wallet inactive")]
    WalletInactive,

    #[error("NFT expired")]
    NFTExpired,

    #[error("Payment overdue")]
    PaymentOverdue,

    #[error("Insufficient collateral for auto debit")]
    InsufficientCollateralForAutoDebit,
}

impl From<FlexfiError> for ProgramError {
    fn from(e: FlexfiError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
