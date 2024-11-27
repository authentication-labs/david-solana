use crate::*;
use oapp::endpoint_cpi::{get_accounts_for_clear, get_accounts_for_send_compose, LzAccount};
use oapp::{endpoint::ID as ENDPOINT_ID, LzReceiveParams};
const REMOTE_SEED: &[u8] = b"Remote";
const COUNT_SEED: &[u8] = b"Count";

#[derive(Accounts)]
pub struct LzReceiveTypes<'info> {
    #[account(seeds = [COUNT_SEED, &factory.key().to_bytes()], bump)]
    pub factory: Account<'info, Factory>,
}

impl LzReceiveTypes<'_> {
    pub fn apply(
        ctx: &Context<LzReceiveTypes>,
        params: &LzReceiveParams,
    ) -> Result<Vec<LzAccount>> {
        let factory = ctx.accounts.factory.key();

        let seeds = [REMOTE_SEED, &factory.to_bytes(), &params.src_eid.to_be_bytes()];
        let (remote, _) = Pubkey::find_program_address(&seeds, ctx.program_id);

        let mut accounts = vec![
            LzAccount { pubkey: factory, is_signer: false, is_writable: true },
            LzAccount { pubkey: remote, is_signer: false, is_writable: false },
        ];

        let accounts_for_clear = get_accounts_for_clear(
            ENDPOINT_ID,
            &factory,
            params.src_eid,
            &params.sender,
            params.nonce,
        );
        accounts.extend(accounts_for_clear);

        let is_composed = msg_codec::msg_type(&params.message) == msg_codec::COMPOSED_TYPE;
        if is_composed {
            let accounts_for_composing = get_accounts_for_send_compose(
                ENDPOINT_ID,
                &factory,
                &factory,
                &params.guid,
                0,
                &params.message,
            );
            accounts.extend(accounts_for_composing);
        }

        Ok(accounts)
    }
}