use pinocchio::error::ProgramError;
use thiserror::Error;

#[repr(u32)]
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum FundraiserError {
    #[error("The token provided is not the expected token")]
    InvalidToken,

    #[error("Target Met, no longer accepting donations")]
    TargetAlreadyMet,

    #[error("This fundraising is still ongoing")]
    StillOngoing,

    #[error("Contributor has already withdrawn or contributor not found")]
    AlreadyWithdrawnOrNotFound,

    #[error("Fundraising has ended")]
    FundraisingEnded,

    #[error("Invalid amount to raise")]
    InvalidAmount,

    #[error("Target Not Met")]
    TargetNotMet,
}

impl From<FundraiserError> for ProgramError {
    fn from(e: FundraiserError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
