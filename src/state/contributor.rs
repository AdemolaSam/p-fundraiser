use pinocchio::{AccountView, Address, error::ProgramError};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contributor{
    pub amount: [u8; 8]
}