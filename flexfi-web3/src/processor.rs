use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
};

use crate::instructions::{FlexfiInstruction, decode_instruction};
use crate::core::{staking, whitelist};
use crate::bnpl::checker;  // Keep only checker
use crate::card::manager;
use crate::nft::{mint, attach};
use crate::score::{contract as score_contract, query as score_query};
use crate::yield_module::{router, tracker};
use crate::freeze_spend::authorization;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = decode_instruction(instruction_data)?;

    match instruction {
        // New Freeze & Spend instructions (REPLACE old BNPL)
        FlexfiInstruction::InitializeFlexFiAccount { authorized_amount, duration_days } => {
            msg!("Instruction: Initialize FlexFi Account");
            authorization::process_initialize_flexfi_account(
                program_id, accounts, authorized_amount, duration_days
            )
        },
        FlexfiInstruction::FlexFiSpend { amount, merchant } => {
            msg!("Instruction: FlexFi Spend");
            authorization::process_flexfi_spend(program_id, accounts, amount, merchant)
        },
        FlexfiInstruction::RevokeFundsAuthorization => {
            msg!("Instruction: Revoke Funds Authorization");
            authorization::process_revoke_authorization(program_id, accounts)
        },

        // Core instructions
        FlexfiInstruction::InitializeWhitelist => {
            msg!("Instruction: Initialize Whitelist");
            whitelist::process_initialize_whitelist(program_id, accounts)
        },
        FlexfiInstruction::AddToWhitelist { user_pubkey } => {
            msg!("Instruction: Add to Whitelist");
            whitelist::process_add_to_whitelist(program_id, accounts, user_pubkey)
        },
        FlexfiInstruction::RemoveFromWhitelist { user_pubkey } => {
            msg!("Instruction: Remove from Whitelist");
            whitelist::process_remove_from_whitelist(program_id, accounts, user_pubkey)
        },

        FlexfiInstruction::DepositStaking { amount, lock_days } => {
            msg!("Instruction: Deposit Staking");
            staking::process_deposit_staking(program_id, accounts, amount, lock_days)
        },
        FlexfiInstruction::WithdrawStaking { amount } => {
            msg!("Instruction: Withdraw Staking");
            staking::process_withdraw_staking(program_id, accounts, amount)
        },

        // NFT instructions
        FlexfiInstruction::MintNFT { nft_type } => {
            msg!("Instruction: Mint NFT");
            mint::process_mint_nft(program_id, accounts, nft_type)
        },
        FlexfiInstruction::AttachNFT { card_id } => {
            msg!("Instruction: Attach NFT");
            attach::process_attach_nft(program_id, accounts, card_id)
        },
        FlexfiInstruction::DetachNFT => {
            msg!("Instruction: Detach NFT");
            attach::process_detach_nft(program_id, accounts)
        },

        // Card instructions
        FlexfiInstruction::UpgradeCard { new_card_type } => {
            msg!("Instruction: Upgrade Card");
            manager::process_upgrade_card(program_id, accounts, new_card_type)
        },

        // Score instructions
        FlexfiInstruction::InitializeScore => {
            msg!("Instruction: Initialize Score");
            score_contract::process_initialize_score(program_id, accounts)
        },
        FlexfiInstruction::UpdateScore { change } => {
            msg!("Instruction: Update Score");
            score_contract::process_update_score(program_id, accounts, change)
        },
        FlexfiInstruction::GetScore => {
            msg!("Instruction: Get Score");
            score_query::process_get_score(program_id, accounts)
        },

        // Yield instructions
        FlexfiInstruction::SetYieldStrategy { strategy, auto_reinvest } => {
            msg!("Instruction: Set Yield Strategy");
            router::process_set_yield_strategy(program_id, accounts, strategy, auto_reinvest)
        },
        FlexfiInstruction::RouteYield { amount } => {
            msg!("Instruction: Route Yield");
            router::process_route_yield(program_id, accounts, amount)
        },
        FlexfiInstruction::ClaimYield { amount } => {
            msg!("Instruction: Claim Yield");
            tracker::process_claim_yield(program_id, accounts, amount)
        },
    }
}
