use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum FlexfiError {
    #[error("Le protocole est en pause")]
    ProtocolPaused,
    
    #[error("Le montant dépasse le maximum autorisé")]
    AmountTooHigh,
    
    #[error("Trop de prêts cette année")]
    TooManyLoans,
    
    #[error("Collatéral insuffisant")]
    InsufficientCollateral,
    
    #[error("Débordement arithmétique")]
    MathOverflow,
    
    #[error("Nombre d'échéances invalide")]
    InvalidInstallments,
    
    #[error("Échéance non autorisée pour ce type de carte")]
    InvalidInstallmentForCard,
    
    #[error("Prêt inactif")]
    LoanNotActive,
    
    #[error("Prêt déjà remboursé")]
    LoanAlreadyPaid,
    
    #[error("Période de grâce non expirée")]
    GracePeriodNotExpired,
    
    #[error("Non autorisé")]
    Unauthorized,
    
    #[error("Frais trop élevés")]
    FeeTooHigh,
    
    #[error("Déjà à ce niveau")]
    AlreadyAtThisLevel,
    
    #[error("Type de carte non valide")]
    InvalidCardType,
    
    #[error("Type de NFT non valide")]
    InvalidNFTType,
    
    #[error("Pas de rendement à réclamer")]
    NoYieldToClaim,
    
    #[error("Staking insuffisant")]
    InsufficientStaking,
    
    #[error("Staking non actif")]
    StakingNotActive,
    
    #[error("Staking gelé")]
    StakingFrozen,
    
    #[error("Wallet inactif")]
    WalletInactive,
    
    #[error("NFT expiré")]
    NFTExpired,
    
    #[error("Paiement en retard")]
    PaymentOverdue,
    
    #[error("Collatéral insuffisant pour le débit automatique")]
    InsufficientCollateralForAutoDebit,
}

impl From<FlexfiError> for ProgramError {
    fn from(e: FlexfiError) -> Self {
        ProgramError::Custom(e as u32)
    }
}