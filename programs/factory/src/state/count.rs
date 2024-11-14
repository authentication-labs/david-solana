use crate::*;

#[account]
pub struct Count {
    pub admin: Pubkey,
}

impl Count {
    pub const SIZE: usize = 8 + std::mem::size_of::<Self>();
}

/// LzReceiveTypesAccounts includes accounts that are used in the LzReceiveTypes
/// instruction.
#[account]
pub struct LzReceiveTypesAccounts {
    pub factory: Pubkey,
}

impl LzReceiveTypesAccounts {
    pub const SIZE: usize = 8 + std::mem::size_of::<Self>();
}

/// LzComposeTypesAccounts includes accounts that are used in the LzComposeTypes
/// instruction.
#[account]
pub struct LzComposeTypesAccounts {
    pub factory: Pubkey,
}

impl LzComposeTypesAccounts {
    pub const SIZE: usize = 8 + std::mem::size_of::<Self>();
}