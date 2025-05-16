use solana_program::{
    program_error::ProgramError,
    msg,
    pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum FlexfiInstruction {
    // Core instructions

    DepositStaking {
        amount: u64,
        lock_days: u16,
    },
    WithdrawStaking {
        amount: u64,
    },
    
    // NFT instructions
    MintNFT {
        nft_type: u8,
    },
    AttachNFT {
        card_id: [u8; 32],
    },
    DetachNFT,
    
    // Card instructions
    UpgradeCard {
        new_card_type: u8,
    },
    
    // Score instructions
    InitializeScore,
    UpdateScore {
        change: i16,
    },
    GetScore,
    
    // Yield instructions
    SetYieldStrategy {
        strategy: u8,
        auto_reinvest: bool,
    },
    RouteYield {
        amount: u64,
    },
    ClaimYield {
        amount: u64,
    },

    InitializeWhitelist,
    AddToWhitelist {
        user_pubkey: Pubkey,
    },
    RemoveFromWhitelist {
        user_pubkey: Pubkey,
    },

        
    InitializeFlexFiAccount {
    authorized_amount: u64,
    duration_days: u16,
    },
    RevokeFundsAuthorization,
    FlexFiSpend {
        amount: u64,
        merchant: Pubkey,
    },
}

pub fn decode_instruction(instruction_data: &[u8]) -> Result<FlexfiInstruction, ProgramError> {
    let instruction = FlexfiInstruction::try_from_slice(instruction_data)
        .map_err(|_| {
            msg!("Error: Failed to deserialize instruction data");
            ProgramError::InvalidInstructionData
        })?;
    
    Ok(instruction)
}