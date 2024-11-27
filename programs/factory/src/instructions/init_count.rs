use crate::*;
use oapp::endpoint::{instructions::RegisterOAppParams, state::endpoint, ID as ENDPOINT_ID};
const COUNT_SEED: &[u8] = b"Count";
const LZ_COMPOSE_TYPES_SEED: &[u8] = b"LzComposeTypes";

#[derive(Accounts)]
#[instruction(params: InitCountParams)]
pub struct InitCount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = Factory::calc_size(100, 100),
        seeds = [COUNT_SEED, &params.id.to_be_bytes()],
        bump
    )]
    pub factory: Account<'info, Factory>,
    #[account(
        init,
        payer = payer,
        space = LzReceiveTypesAccounts::SIZE,
        seeds = [LZ_RECEIVE_TYPES_SEED, &factory.key().to_bytes()],
        bump
    )]
    pub lz_receive_types_accounts: Account<'info, LzReceiveTypesAccounts>,
    #[account(
        init,
        payer = payer,
        space = LzComposeTypesAccounts::SIZE,
        seeds = [LZ_COMPOSE_TYPES_SEED, &factory.key().to_bytes()],
        bump
    )]
    pub lz_compose_types_accounts: Account<'info, LzComposeTypesAccounts>,
    pub system_program: Program<'info, System>,
}

impl InitCount<'_> {
    pub fn apply(ctx: &mut Context<InitCount>, params: &InitCountParams) -> Result<()> {
        ctx.accounts.factory.initialized = true;
        ctx.accounts.factory.owner = params.admin;

        ctx.accounts.factory.id = params.id;
        ctx.accounts.factory.bump = ctx.bumps.factory;
        ctx.accounts.factory.endpoint_program = params.endpoint;


        ctx.accounts.lz_receive_types_accounts.factory = ctx.accounts.factory.key();
        ctx.accounts.lz_compose_types_accounts.factory = ctx.accounts.factory.key();

        // calling endpoint cpi
        let register_params = RegisterOAppParams { delegate: ctx.accounts.factory.owner };
        let seeds: &[&[u8]] = &[COUNT_SEED, &[ctx.accounts.factory.id], &[ctx.accounts.factory.bump]];
        oapp::endpoint_cpi::register_oapp(
            ENDPOINT_ID,
            ctx.accounts.factory.key(),
            ctx.remaining_accounts,
            seeds,
            register_params,
        )?;

        Ok(())
    }
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct InitCountParams {
    pub id: u8,
    pub admin: Pubkey,
    pub endpoint: Pubkey,
}