use crate::*;
use oapp::{
    endpoint::{
        cpi::accounts::Clear,
        instructions::ClearParams,
        ConstructCPIContext, ID as ENDPOINT_ID,
    },
    LzReceiveParams,
};
const COUNT_SEED: &[u8] = b"Count";

#[derive(Accounts)]
#[instruction(params: LzReceiveParams)]
pub struct LzReceive<'info> {
    #[account(mut)]
    pub factory: Account<'info, Factory>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(address = identity_lib::ID)]
    pub identity_program: Program<'info, Identity>,
    pub keys_account: Account<'info, KeysAccount>,
    pub claims_account: Account<'info, ClaimsAccount>,
}

impl LzReceive<'_> {
    pub fn apply(ctx: Context<LzReceive>, params: &LzReceiveParams) -> Result<()> {
        let seeds: &[&[u8]] = &[COUNT_SEED, &ctx.accounts.factory.key().to_bytes()];

        // the first 9 accounts are for clear()
        let accounts_for_clear = &ctx.remaining_accounts[0..Clear::MIN_ACCOUNTS_LEN];
        let _ = oapp::endpoint_cpi::clear(
            ENDPOINT_ID,
            ctx.accounts.factory.key(),
            accounts_for_clear,
            seeds,
            ClearParams {
                receiver: ctx.accounts.factory.key(),
                src_eid: params.src_eid,
                sender: params.sender,
                nonce: params.nonce,
                guid: params.guid,
                message: params.message.clone(),
            },
        )?;

        let (method_name, payload) = decode_message(&params.message)?;

        match method_name.as_str() {
            "CreateIdentity" => {
                let (identity_owner, salt) = decode_create_identity_payload(&payload)?;
                let initial_management_key = ctx.accounts.factory.owner; 
                create_identity(ctx, identity_owner, salt, initial_management_key)?;
            }
            "AddKey" => {
                let (wallet, key, purpose, key_type) = decode_add_key_payload(&payload)?;
                add_key(ctx, wallet, key, purpose, key_type)?;
            }
            "AddClaim" => {
                let (wallet, topic, scheme, issuer_wallet, signature, data, uri) =
                    decode_add_claim_payload(&payload)?;
                add_claim(ctx, wallet, topic, scheme, issuer_wallet, signature, data, uri)?;
            }
            "RemoveKey" => {
                let (wallet, key, purpose) = decode_remove_key_payload(&payload)?;
                remove_key(ctx, wallet, key, purpose)?;
            }
            "RemoveClaim" => {
                let (wallet, topic) = decode_remove_claim_payload(&payload)?;
                remove_claim(ctx, wallet, topic)?;
            }
            _ => return Err(ProgramError::InvalidInstructionData.into()),
        }

        Ok(())
    }
}