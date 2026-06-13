#![allow(unexpected_cfgs)]

use pinocchio::{
    AccountView, Address, ProgramResult, address::declare_id, entrypoint, error::ProgramError,
};

use crate::instructions::FundraiserInstructions;

mod constants;
mod error;
mod instructions;
mod state;

entrypoint!(process_instruction);

declare_id!("BHxV2zsi55UNqDL4ns2e6iyQWTJj78qeyRYcc9N4RoT1");

fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(program_id, &ID);

    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match FundraiserInstructions::try_from(discriminator)? {
        FundraiserInstructions::Initialize => instructions::process_initialize(accounts, data),

        FundraiserInstructions::Contribute => instructions::process_contribute(accounts, data),

        FundraiserInstructions::Refund => instructions::process_refund(accounts, data),

        FundraiserInstructions::Withdraw => instructions::process_withdraw(accounts, data),
    }
}
