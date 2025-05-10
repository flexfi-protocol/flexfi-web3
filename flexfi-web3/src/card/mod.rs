pub mod config;
pub mod manager;

pub use config::{get_card_annual_fee, is_installment_allowed_for_card, get_max_installments_for_card};
pub use manager::process_upgrade_card;