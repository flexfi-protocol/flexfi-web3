use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, Sysvar, rent::Rent},
    msg,
};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::error::FlexfiError;
use crate::state::wallet::WalletAccount;
use crate::state::card::CardAccount;
use crate::constants::{CARD_STANDARD, CARD_SILVER, CARD_GOLD, CARD_PLATINUM, CARD_SEED};
use crate::card::config::get_card_annual_fee;
use crate::core::whitelist::require_whitelisted;

pub fn process_upgrade_card(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_card_type: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let wallet_account = next_account_info(account_info_iter)?;
    let card_account = next_account_info(account_info_iter)?;
    let user_account = next_account_info(account_info_iter)?;
    let user_status_account = next_account_info(account_info_iter)?;
    let user_token_account = next_account_info(account_info_iter)?;
    let fee_account = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let clock_sysvar = next_account_info(account_info_iter)?;

    // Check user signature
    if !user_account.is_signer {
        return Err(FlexfiError::Unauthorized.into());
    }

    require_whitelisted(
        program_id,
        user_account.key,
        user_status_account
    )?;

    // Check if the card type is valid
    if new_card_type > CARD_PLATINUM {
        return Err(FlexfiError::InvalidCardType.into());
    }

    // Load wallet data
    let mut wallet_data = WalletAccount::try_from_slice(&wallet_account.data.borrow())?;

    // Verify that the user is the owner of the wallet
    if wallet_data.owner != *user_account.key {
        return Err(FlexfiError::Unauthorized.into());
    }

    // Verify that the wallet is active
    if !wallet_data.is_active {
        return Err(FlexfiError::WalletInactive.into());
    }

    // Verify that the new card type is different and higher
    if wallet_data.card_type == new_card_type {
        return Err(FlexfiError::AlreadyAtThisLevel.into());
    }

    if wallet_data.card_type > new_card_type {
        return Err(ProgramError::InvalidArgument);
    }

    // Calculate upgrade fees
    let current_fee = get_card_annual_fee(wallet_data.card_type)?;
    let new_fee = get_card_annual_fee(new_card_type)?;

    let upgrade_fee = new_fee.saturating_sub(current_fee);

    // Get current timestamp
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Create or update the card account
    if card_account.owner == program_id {
        // Update existing card
        let mut card_data = CardAccount::try_from_slice(&card_account.data.borrow())?;

        // Verify that the user is the owner
        if card_data.owner != *user_account.key {
            return Err(FlexfiError::Unauthorized.into());
        }

        // Update card type
        card_data.card_type = new_card_type;

        // Update annual fee expiration date
        card_data.annual_fee_paid_until = current_time + (365 * 86400);

        // Save changes
        card_data.serialize(&mut *card_account.data.borrow_mut())?;
    } else {
        // Create a new card account
        let seeds = [
            CARD_SEED,
            user_account.key.as_ref(),
        ];
        let (card_pda, card_bump) = Pubkey::find_program_address(&seeds, program_id);

        if *card_account.key != card_pda {
            return Err(ProgramError::InvalidAccountData);
        }

        // Create the account
        let rent = Rent::get()?;
        let space = CardAccount::SIZE;
        let rent_lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                user_account.key,
                &card_pda,
                rent_lamports,
                space as u64,
                program_id,
            ),
            &[user_account.clone(), card_account.clone(), system_program.clone()],
            &[&[CARD_SEED, user_account.key.as_ref(), &[card_bump]]],
        )?;

        // Initialize card data
        let card_data = CardAccount::new(
            *user_account.key,
            new_card_type,
            current_time,
            card_bump,
        );

        // Save data
        card_data.serialize(&mut *card_account.data.borrow_mut())?;
    }

    // Update card type in the wallet
    wallet_data.card_type = new_card_type;
    wallet_data.serialize(&mut *wallet_account.data.borrow_mut())?;

    // Transfer upgrade fees if necessary
    if upgrade_fee > 0 {
        let transfer_ix = spl_token::instruction::transfer(
            token_program.key,
            user_token_account.key,
            fee_account.key,
            user_account.key,
            &[],
            upgrade_fee,
        )?;

        invoke(
            &transfer_ix,
            &[
                user_token_account.clone(),
                fee_account.clone(),
                user_account.clone(),
                token_program.clone(),
            ],
        )?;
    }

    msg!("Card upgraded to type {}", new_card_type);
    Ok(())
}

pub struct CardManager;

impl CardManager {
    pub fn upgrade_card(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        new_card_type: u8,
    ) -> ProgramResult {
        process_upgrade_card(program_id, accounts, new_card_type)
    }
}
