use solana_program::{
    program_error::ProgramError,
    msg,
};

use crate::constants::{CARD_STANDARD, CARD_SILVER, CARD_GOLD, CARD_PLATINUM, get_card_config};

pub fn get_card_annual_fee(card_type: u8) -> Result<u64, ProgramError> {
    match card_type {
        CARD_STANDARD => Ok(0),                    // Gratuit
        CARD_SILVER => Ok(50_000_000),            // 50 USDC
        CARD_GOLD => Ok(150_000_000),             // 150 USDC
        CARD_PLATINUM => Ok(300_000_000),         // 300 USDC
        _ => {
            msg!("Type de carte invalide: {}", card_type);
            Err(ProgramError::InvalidArgument)
        }
    }
}

pub fn is_installment_allowed_for_card(card_type: u8, installment: u8) -> bool {
    let card_config = get_card_config(card_type);
    card_config.available_installments.contains(&installment)
}

pub fn get_max_installments_for_card(card_type: u8) -> u8 {
    let card_config = get_card_config(card_type);
    card_config.max_installments
}