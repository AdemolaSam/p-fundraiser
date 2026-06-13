use crate::{constants::MIN_AMOUNT_TO_RAISE, error::FundraiserError, state::Fundraiser};
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

pub fn process_initialize(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser_account,
        system_program,
        token_program,
        vault_ata,
        _associated_token_program @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() != 10 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // extract data / args
    let bump = data[0];
    let duration = data[1];
    let amount_to_raise = u64::from_le_bytes(
        data[2..10]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    let seed = [b"fundraiser".as_ref(), maker.address().as_ref(), &[bump]];

    let fundraiser_pda = derive_address(&seed, None, &crate::ID.to_bytes());

    let bump_bytes = [bump];

    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);

    //checks
    {
        if fundraiser_account.address().as_array() != &fundraiser_pda {
            return Err(ProgramError::InvalidAccountData);
        }

        let mint_state = pinocchio_token::state::Mint::from_account_view(mint_to_raise)?;
        let min_amount = MIN_AMOUNT_TO_RAISE * 10_u64.pow(mint_state.decimals() as u32);
        if amount_to_raise < min_amount {
            return Err(FundraiserError::InvalidAmount.into());
        }

        if !mint_to_raise.owned_by(token_program.address()) {
            return Err(ProgramError::IllegalOwner);
        }
    }

    // CreateAccount
    CreateAccount {
        from: maker,
        to: fundraiser_account,
        lamports: Rent::get()?.try_minimum_balance(Fundraiser::LEN)?,
        space: Fundraiser::LEN as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[signer])?;

    //current timestamp
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;

    let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
    fundraiser_state.set_maker(maker.address());
    fundraiser_state.set_amount_to_raise(amount_to_raise);
    fundraiser_state.set_duration(duration);
    fundraiser_state.set_current_amount(0);
    fundraiser_state.set_time_started(current_timestamp);
    fundraiser_state.set_mint_to_raise(mint_to_raise.address());
    fundraiser_state.set_bump(bump);

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault_ata,
        wallet: fundraiser_account,
        mint: mint_to_raise,
        token_program,
        system_program,
    }
    .invoke()?;

    Ok(())
}
