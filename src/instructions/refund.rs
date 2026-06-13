use crate::{
    error::FundraiserError,
    state::{Contributor, Fundraiser},
};
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, clock::Clock},
};
use pinocchio_pubkey::derive_address;

pub fn process_refund(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        contributor,
        contributor_ata,
        contributor_account,
        fundraiser_account,
        mint_to_raise,
        _system_program,
        _token_program,
        vault_ata,
        _associated_token_program @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if data.len() != 2 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let bump = data[0];
    let contributor_bump = data[1];
    let seed = [b"fundraiser", maker.address().as_ref(), &[bump]];
    let fundraiser_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    let contributor_seed = [
        b"contributor",
        fundraiser_pda.as_ref(),
        contributor.address().as_ref(),
        &[contributor_bump],
    ];
    let contributor_pda = derive_address(&contributor_seed, None, &crate::ID.to_bytes());
    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(bump_bytes.as_ref()),
    ];

    let signer = Signer::from(&signer_seeds);

    {
        // checks
        let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
        //check if fundraising has closed
        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp;

        let fundraiser_time =
            fundraiser_state.time_started() + (fundraiser_state.duration() as i64);

        if current_timestamp <= fundraiser_time {
            return Err(FundraiserError::StillOngoing.into());
        }

        //ensure contributor is signer
        if !contributor.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        //correct token
        if mint_to_raise.address() != fundraiser_state.mint_to_raise() {
            return Err(FundraiserError::InvalidToken.into());
        }

        //contributor ATA (refund ATA) matches the fundraiser mint
        let contributor_ata_state =
            pinocchio_token::state::Account::from_account_view(contributor_ata)?;
        if contributor_ata_state.mint() != fundraiser_state.mint_to_raise() {
            return Err(FundraiserError::InvalidToken.into());
        }
        //contributor ATA is owned by contributor
        if contributor_ata_state.owner() != contributor.address() {
            return Err(ProgramError::IllegalOwner);
        }
        //check valid contributor PDA
        if contributor_account.address().as_array() != &contributor_pda {
            return Err(FundraiserError::AlreadyWithdrawnOrNotFound.into());
        }

        //vault ATA is same as fundraise mint
        let vault_ata_state = pinocchio_token::state::Account::from_account_view(vault_ata)?;
        if vault_ata_state.mint() != fundraiser_state.mint_to_raise() {
            return Err(FundraiserError::InvalidToken.into());
        }

        //vault ATA is owned by Fundraiser PDA
        if vault_ata_state.owner().as_array() != &fundraiser_pda {
            return Err(ProgramError::IllegalOwner);
        }
    }

    let contributor_state = Contributor::from_account_info(contributor_account)?;
    let mint_data = pinocchio_token::state::Mint::from_account_view(mint_to_raise)?;
    let refund_amount = contributor_state.amount();

    if refund_amount == 0 {
        return Err(FundraiserError::AlreadyWithdrawnOrNotFound.into());
    }

    pinocchio_token::instructions::TransferChecked::new(
        vault_ata,
        mint_to_raise,
        contributor_ata,
        fundraiser_account,
        refund_amount,
        mint_data.decimals(),
    )
    .invoke_signed(&[signer])?;

    let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
    let current_amount = fundraiser_state
        .current_amount()
        .checked_sub(refund_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    fundraiser_state.set_current_amount(current_amount);

    Contributor::close_contributor_pda(contributor_account, contributor);

    Ok(())
}
