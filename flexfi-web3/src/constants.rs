// General constants for the FlexFi program
pub const FLEXFI_VERSION: &str = "1.0.0";
pub const PROGRAM_AUTHORITY_SEED: &[u8] = b"program_authority";

// Card types
pub const CARD_STANDARD: u8 = 0;
pub const CARD_SILVER: u8 = 1;
pub const CARD_GOLD: u8 = 2;
pub const CARD_PLATINUM: u8 = 3;

// NFT types
pub const NFT_NONE: u8 = 0;
pub const NFT_BRONZE: u8 = 1;
pub const NFT_SILVER: u8 = 2;
pub const NFT_GOLD: u8 = 3;

// Fee-related constants
pub const DEFAULT_FEE_PERCENTAGE: u16 = 700; // 7.00%
pub const MINIMUM_FEE_PERCENTAGE: u16 = 300;  // 3.00%
pub const MAXIMUM_FEE_PERCENTAGE: u16 = 700; // 7.00%

// NFT minting cost
pub const NFT_MINT_COST: u64 = 20_000_000; // 20 USDC (with 6 decimals)

// Card configurations (APR, BNPL fees, installments)
pub struct CardConfig {
    pub apr_percentage: u16,           // APR in basis points (e.g., 400 = 4%)
    pub bnpl_fee_percentage: u16,      // BNPL fees in basis points
    pub bnpl_fee_12months: u16,        // BNPL fees for 12 months
    pub max_installments: u8,          // Maximum number of installments
    pub available_installments: [u8; 4], // Available installments (3,4,6,12)
    pub cashback_percentage: u16,      // Cashback in basis points
    pub cashback_limit: u64,           // Monthly cashback limit in USDC (with 6 decimals)
    pub nft_cost: u64,                 // NFT cost in USDC (with 6 decimals)
}

// Get the configuration of a card
pub fn get_card_config(card_type: u8) -> CardConfig {
    match card_type {
        CARD_STANDARD => CardConfig {
            apr_percentage: 400,           // 4%
            bnpl_fee_percentage: 700,      // 7%
            bnpl_fee_12months: 700,        // 7% for 12 months as well
            max_installments: 6,           // 6 months max
            available_installments: [3, 4, 6, 0],
            cashback_percentage: 0,        // No cashback
            cashback_limit: 0,             // No limit
            nft_cost: 0,                   // Standard does not include NFT
        },
        CARD_SILVER => CardConfig {
            apr_percentage: 500,           // 5%
            bnpl_fee_percentage: 400,      // 4%
            bnpl_fee_12months: 700,        // 7% for 12 months
            max_installments: 12,          // 12 months max
            available_installments: [3, 4, 6, 12],
            cashback_percentage: 0,        // No cashback
            cashback_limit: 0,             // No limit
            nft_cost: 20_000_000,          // 20 USDC
        },
        CARD_GOLD => CardConfig {
            apr_percentage: 600,           // 6%
            bnpl_fee_percentage: 350,      // 3.5%
            bnpl_fee_12months: 500,        // 5% for 12 months
            max_installments: 12,          // 12 months max
            available_installments: [3, 4, 6, 12],
            cashback_percentage: 50,       // 0.5%
            cashback_limit: 150_000_000,   // 150 USDC
            nft_cost: 15_000_000,          // 15 USDC
        },
        CARD_PLATINUM => CardConfig {
            apr_percentage: 700,           // 7%
            bnpl_fee_percentage: 300,      // 3%
            bnpl_fee_12months: 300,        // 3% for 12 months as well
            max_installments: 12,          // 12 months max
            available_installments: [3, 4, 6, 12],
            cashback_percentage: 150,      // 1.5%
            cashback_limit: 300_000_000,   // 300 USDC
            nft_cost: 0,                   // NFT included
        },
        _ => CardConfig {                  // Default value (Standard)
            apr_percentage: 400,
            bnpl_fee_percentage: 700,
            bnpl_fee_12months: 700,
            max_installments: 6,
            available_installments: [3, 4, 6, 0],
            cashback_percentage: 0,
            cashback_limit: 0,
            nft_cost: 0,
        },
    }
}

// Get the APR bonus for a combination with NFT
pub fn get_nft_apr_bonus(nft_type: u8) -> u16 {
    match nft_type {
        NFT_BRONZE => 50,   // +0.5%
        NFT_SILVER => 150,  // +1.5%
        NFT_GOLD => 200,    // +2%
        _ => 0,             // No bonus
    }
}

// Get late payment penalty fees based on card+NFT combination
pub fn get_late_payment_penalty(card_type: u8, nft_type: u8) -> u16 {
    match (card_type, nft_type) {
        (CARD_SILVER, NFT_BRONZE) => 700,  // 7%
        (CARD_SILVER, NFT_SILVER) => 600,  // 6%
        (CARD_GOLD, NFT_BRONZE) => 500,    // 5%
        (CARD_GOLD, NFT_SILVER) => 400,    // 4%
        (CARD_PLATINUM, NFT_BRONZE) => 200, // 2%
        (CARD_PLATINUM, NFT_GOLD) => 100,  // 1%
        _ => 1000,                         // 10% default
    }
}

// BNPL-related constants
pub const MIN_BNPL_INSTALLMENTS: u8 = 3;
pub const MAX_BNPL_INSTALLMENTS: u8 = 36;
pub const MIN_PAYMENT_INTERVAL_DAYS: u8 = 15;
pub const MAX_PAYMENT_INTERVAL_DAYS: u8 = 90;
pub const DEFAULT_PAYMENT_INTERVAL_DAYS: u8 = 30;
pub const GRACE_PERIOD_DAYS: u8 = 15;
pub const MAX_BNPL_PER_YEAR: u16 = 5;

// Staking-related constants
pub const MIN_STAKING_AMOUNT: u64 = 10_000_000; // 10 USDC (with 6 decimals)
pub const MIN_STAKING_LOCK_DAYS: u16 = 7;
pub const MAX_STAKING_LOCK_DAYS: u16 = 365;

// Scoring-related constants
pub const INITIAL_SCORE: u16 = 500;
pub const MIN_SCORE: u16 = 0;
pub const MAX_SCORE: u16 = 1000;
pub const SCORE_INCREASE_ON_TIME_PAYMENT: i16 = 5;
pub const SCORE_DECREASE_LATE_PAYMENT: i16 = -10;
pub const SCORE_DECREASE_DEFAULT: i16 = -50;
pub const SCORE_INCREASE_COMPLETE_CONTRACT: i16 = 20;

// PDA Seeds
pub const WALLET_SEED: &[u8] = b"wallet";
pub const BACKEND_ID_SEED: &[u8] = b"backend_id";
pub const STAKING_SEED: &[u8] = b"staking";
pub const USDC_VAULT_SEED: &[u8] = b"usdc_vault";
pub const BNPL_CONTRACT_SEED: &[u8] = b"bnpl_contract";
pub const SCORE_SEED: &[u8] = b"score";
pub const YIELD_CONFIG_SEED: &[u8] = b"yield_config";
pub const YIELD_VAULT_SEED: &[u8] = b"yield_vault";
pub const YIELD_TRACKER_SEED: &[u8] = b"yield_tracker";
pub const NFT_METADATA_SEED: &[u8] = b"nft_metadata";
pub const NFT_ATTACHMENT_SEED: &[u8] = b"nft_attachment";
pub const CARD_SEED: &[u8] = b"card";

pub const WHITELIST_SEED: &[u8] = b"whitelist";
pub const ADMIN_LIST_SEED: &[u8] = b"admin_list";

pub const AUTHORIZATION_SEED: &[u8] = b"authorization";

pub const FLEXFI_AUTHORITY_SEED: &[u8] = b"flexfi_authority";
