use anchor_lang::{prelude::*, solana_program::instruction};

declare_id!("8DXtpG31GL4L215EeREcPhQCFgFjWcWQjX27d9XEFsRo");

#[program]
pub mod identiy {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
    pub fn add_key(_ctx: Context<AddKey>) -> Result<()> {
        Ok(())
    }
    pub fn add_claim(_ctx: Context<AddClaim>) -> Result<()> {
        Ok(())
    }
    pub fn is_claim_valid(_ctx: Context<IsClaimValid>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct AddKey {}

#[derive(Accounts)]
pub struct AddClaim {}

#[derive(Accounts)]
pub struct IsClaimValid {}

#[account]
pub struct IdentityAccount {}

#[account]
pub struct KeyAccount {
    key: Vec<u32>,
    purpose: u8,
}

#[account]
pub struct ClaimAccount {
    claim_topc: u32,
    scheme: u32,
    issuer: Pubkey,
    signature: [u8; 64], // TODO: change – depends on signature size
    data: Vec<u8>,
    uri: String,
}
