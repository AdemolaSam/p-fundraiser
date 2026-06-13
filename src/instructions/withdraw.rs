use crate::state::Fundraiser;
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_pubkey::derive_address;

use crate::error::FundraiserError;

pub fn process_withdraw(accounts: &mut [AccountView], _data: &[u8]) -> ProgramResult {
    let [
        maker,
        maker_ata,
        fundraiser_account,
        mint_to_raise,
        vault_ata,
        _system_program,
        token_program,
        _associated_token_program @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let fundraiser_account_key = *fundraiser_account.address().as_array();

    {
        //checks
        let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
        //perfom checks here
        // check that the withdrawer is the maker
        if fundraiser_state.maker() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }

        //maker is signer
        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // check that maker_ata (the withdrawer's ATA) is owned by the maker
        let maker_ata_state = pinocchio_token::state::Account::from_account_view(maker_ata)?;
        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        // check that the vault ATA is owned by the system program
        if !vault_ata.owned_by(token_program.address()) {
            return Err(ProgramError::InvalidAccountData);
        }

        let bump = fundraiser_state.bump;
        let seed = [b"fundraiser".as_ref(), maker.address().as_ref(), &[bump]];
        let fundraiser_pda = derive_address(&seed, None, &crate::ID.to_bytes());

        //correct PDA address
        if fundraiser_pda != fundraiser_account_key {
            return Err(ProgramError::InvalidAccountData);
        }

        //is target amount met?
        let current_amount = fundraiser_state.current_amount();
        if current_amount < fundraiser_state.amount_to_raise() {
            return Err(FundraiserError::TargetNotMet.into());
        }

        // correct mint
        if mint_to_raise.address() != fundraiser_state.mint_to_raise() {
            return Err(FundraiserError::InvalidToken.into());
        }
        if maker_ata_state.mint() != fundraiser_state.mint_to_raise() {
            return Err(FundraiserError::InvalidToken.into());
        }

        let vault_ata_state = pinocchio_token::state::Account::from_account_view(vault_ata)?;
        if vault_ata_state.mint() != fundraiser_state.mint_to_raise() {
            return Err(FundraiserError::InvalidToken.into());
        }
        if vault_ata_state.owner().as_array() != &fundraiser_pda {
            return Err(ProgramError::IllegalOwner);
        }
    }

    let fundraiser_state = Fundraiser::from_account_info(fundraiser_account)?;
    let bump = fundraiser_state.bump;
    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(bump_bytes.as_ref()),
    ];

    let signer = Signer::from(&signer_seeds);

    let amount = fundraiser_state.current_amount();

    pinocchio_token::instructions::Transfer::new(vault_ata, maker_ata, fundraiser_account, amount)
        .invoke_signed(&[signer])?;

    //close vault token account (ATA)
    pinocchio_token::instructions::CloseAccount::new(vault_ata, maker, fundraiser_account)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    //close the fundraiser account
    Fundraiser::close_fundraiser_pda(fundraiser_account, maker);

    Ok(())
}
