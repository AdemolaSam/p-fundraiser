use crate::{
    error::FundraiserError,
    state::{Contributor, Fundraiser},
};
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

pub fn process_contribute(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        fundraiser_account,
        contributor,
        contributor_account,
        contributor_ata,
        vault_ata,
        _system_program,
        _token_program,
        _associated_token_account @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() != 10 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let bump = data[0];
    let contributor_bump = data[1];
    let amount = u64::from_le_bytes(
        data[2..10]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    let seed = [b"fundraiser", maker.address().as_ref(), &[bump]];
    let fundraiser_pda = derive_address(&seed, None, &crate::ID.to_bytes());

    //contributor pda seed
    let contributor_seed = [
        b"contributor",
        fundraiser_pda.as_ref(),
        contributor.address().as_ref(),
        &[contributor_bump],
    ];
    let contributor_pda = derive_address(&contributor_seed, None, &crate::ID.to_bytes());

    //checks
    {
        let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
        //correct token mint
        let contributor_ata_state =
            pinocchio_token::state::Account::from_account_view(contributor_ata)?;
        let vault_ata_state = pinocchio_token::state::Account::from_account_view(vault_ata)?;

        if contributor_ata_state.mint() != fundraiser_state.mint_to_raise() {
            return Err(FundraiserError::InvalidToken.into());
        }

        //contributor owns the contributor ATA
        if contributor_ata_state.owner() != contributor.address() {
            return Err(ProgramError::IllegalOwner);
        }

        //validate vault ATA
        if fundraiser_state.mint_to_raise() != vault_ata_state.mint() {
            return Err(FundraiserError::InvalidToken.into());
        }

        //maker owns the fundraiser
        if fundraiser_state.maker() != maker.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        //Ensure target not met
        if fundraiser_state.current_amount() >= fundraiser_state.amount_to_raise() {
            return Err(ProgramError::InvalidAccountData);
        }

        //check if fundraising is still ongoing
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;
        let fundraiser_endtime =
            (fundraiser_state.duration() as i64) + fundraiser_state.time_started();
        if current_time > fundraiser_endtime {
            return Err(FundraiserError::FundraisingEnded.into());
        }

        //check valid contributor
        if !contributor.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
    }

    // check if valid fundraiser
    if fundraiser_account.address().as_array() != &fundraiser_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // check if valid contributor PDA
    if contributor_account.address().as_array() != &contributor_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    let contributor_bump_bytes = [contributor_bump];
    let signer_seeds = [
        Seed::from(b"contributor"),
        Seed::from(fundraiser_pda.as_ref()),
        Seed::from(contributor.address().as_ref()),
        Seed::from(contributor_bump_bytes.as_ref()),
    ];

    let signer = Signer::from(&signer_seeds);
    //create contributor account to store amount contributed
    // check if contributor account already exists
    if !contributor_account.owned_by(&crate::ID) {
        CreateAccount {
            from: contributor,
            to: contributor_account,
            lamports: Rent::get()?.try_minimum_balance(Contributor::LEN)?,
            space: Contributor::LEN as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[signer])?;
    }

    pinocchio_token::instructions::Transfer::new(contributor_ata, vault_ata, contributor, amount)
        .invoke()?;

    //update state
    let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
    let contributor_state = Contributor::from_account_info(contributor_account)?;
    let previous_current = fundraiser_state.current_amount();
    let new_current = previous_current
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    fundraiser_state.set_current_amount(new_current);

    let previous_contribution = contributor_state.amount();
    let new_contribution = previous_contribution
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    contributor_state.set_amount(new_contribution);

    Ok(())
}
