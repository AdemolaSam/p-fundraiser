pub mod initialize;
pub use initialize::*;

pub mod withdraw;
pub use withdraw::*;

pub mod contribute;
pub use contribute::*;

pub mod refund;
pub use refund::*;

use pinocchio::error::ProgramError;
pub enum FundraiserInstructions {
    Initialize,
    Contribute,
    Refund,
    Withdraw,
}

impl TryFrom<&u8> for FundraiserInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundraiserInstructions::Initialize),
            1 => Ok(FundraiserInstructions::Contribute),
            2 => Ok(FundraiserInstructions::Refund),
            3 => Ok(FundraiserInstructions::Withdraw),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
