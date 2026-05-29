use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;
use solana_sdk_ids::system_program;

pub fn process_initialize(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser,
        system_program,
        token_program,
        vault_ata,
        _associated_token_program @ ..,

    ] = accounts
    else {
       return Err(ProgramError::NotEnoughAccountKeys)
    };

    let bump = data[0];
    let seeds = [b"fundraiser".as_ref(), maker.address().as_ref(), &[bump]];

    let fundraiser_pda = derive_address(&seeds, None, &crate::ID.to_bytes());





    Ok(())
}