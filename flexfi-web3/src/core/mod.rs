pub mod staking;
pub mod whitelist;

pub use staking::{process_deposit_staking, process_withdraw_staking};
pub use whitelist::{
    process_initialize_whitelist, 
    process_add_to_whitelist,
    process_remove_from_whitelist,
    check_user_whitelisted, 
    require_whitelisted
};