use crate::*;
use anchor_lang::prelude::*;

#[account]
pub struct Remote {
    pub address: [u8; 32],
    pub bump: u8,
}

impl Remote {
    pub const SIZE: usize = 8 + std::mem::size_of::<Self>();
}

#[derive(Accounts)]
#[instruction(params: SetRemoteParams)]
pub struct SetRemote<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init_if_needed,
        payer = admin,
        space = Remote::SIZE
    )]
    pub remote: Account<'info, Remote>,
    pub system_program: Program<'info, System>,
}

impl SetRemote<'_> {
    pub fn apply(ctx: &mut Context<SetRemote>, params: &SetRemoteParams) -> Result<()> {
        ctx.accounts.remote.address = params.remote;
        Ok(())
    }
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SetRemoteParams {
    pub dst_eid: u32,
    pub remote: [u8; 32],
}
