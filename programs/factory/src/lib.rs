use anchor_lang::{prelude::*, solana_program};
use solana_program::{instruction::{Instruction, AccountMeta}, program::invoke_signed, pubkey::Pubkey};
use identity_lib::program::Identity;

declare_id!("Fg6PaFpoGXkYsidMpWFK1THCyGDMhJWAXR2ZsD6xXc6C");

#[program]
pub mod factory_contract {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let factory = &mut ctx.accounts.factory;

        require!(!factory.initialized, ErrorCode::AlreadyInitialized);

        factory.initialized = true;
        factory.owner = *ctx.accounts.payer.key;

        msg!("Factory contract initialized by owner {:?}.", factory.owner);
        emit!(FactoryInitialized {
            owner: factory.owner,
        });

        Ok(())
    }

    pub fn get_initialized(ctx: Context<Initialize>) -> Result<bool> {
        let factory = &ctx.accounts.factory;
        Ok(factory.initialized)
    }

    pub fn create_identity(
        ctx: Context<CreateIdentity>,
        wallet: Pubkey,
        salt: [u8; 32],
        initial_management_key: Pubkey,
    ) -> Result<()> {
        require!(*ctx.accounts.payer.key == ctx.accounts.factory.owner, ErrorCode::Unauthorized);

        let seeds = &[b"identity", wallet.as_ref(), salt.as_ref()];
        let (identity_address, bump) = Pubkey::find_program_address(seeds, ctx.program_id);

        let instruction = Instruction {
            program_id: ctx.accounts.identity_program.key(),
            accounts: vec![
                AccountMeta::new(identity_address, false),
                AccountMeta::new(*ctx.accounts.payer.key, false),
                AccountMeta::new(ctx.accounts.factory.key(), false),
                AccountMeta::new_readonly(*ctx.accounts.system_program.key, false),
            ],
            data: initial_management_key.to_bytes().to_vec(),
        };

        let signers_seeds: &[&[&[u8]]] = &[&[b"identity", wallet.as_ref(), salt.as_ref(), &[bump]]];

        invoke_signed(
            &instruction,
            &[
                ctx.accounts.factory.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.identity_program.to_account_info(),
            ],
            signers_seeds,
        )?;

        ctx.accounts.factory.wallet_to_identity.push((wallet, identity_address));
        ctx.accounts.factory.linked_wallets.push(wallet);
        ctx.accounts.factory.identity_addresses.push(identity_address);
        msg!("Identity created with address: {:?}", identity_address);
        emit!(IdentityCreated {
            wallet,
            identity_address,
        });

        Ok(())
    }

    pub fn link_wallet(
        ctx: Context<LinkedWallets>,
        wallet: Pubkey,
        identity: Pubkey,
    ) -> Result<()> {
        let factory_account = &mut ctx.accounts.factory;

        require!(ctx.accounts.signer.key == &factory_account.owner, ErrorCode::Unauthorized);

        if !factory_account.linked_wallets.contains(&wallet) {
            factory_account.linked_wallets.push(wallet);
            factory_account.wallet_to_identity.push((wallet, identity));
            msg!("Wallet {:?} linked with identity {:?}", wallet, identity);
            emit!(WalletLinked {
                wallet,
                identity,
            });
        } else {
            msg!("Wallet is already linked to this identity.");
        }

        Ok(())
    }

    pub fn unlink_wallet(
        ctx: Context<LinkedWallets>,
        wallet: Pubkey,
        identity: Pubkey,
    ) -> Result<()> {
        let factory_account = &mut ctx.accounts.factory;

        require!(ctx.accounts.signer.key == &factory_account.owner, ErrorCode::Unauthorized);

        if let Some(index) = factory_account.linked_wallets.iter().position(|&x| x == wallet) {
            factory_account.linked_wallets.remove(index);
            if let Some(pos) = factory_account.wallet_to_identity.iter().position(|&(w, _)| w == wallet) {
                factory_account.wallet_to_identity.remove(pos);
            }
            msg!("Wallet {:?} unlinked from identity {:?}", wallet, identity);
            emit!(WalletUnlinked {
                wallet,
                identity,
            });
        } else {
            msg!("Wallet not linked to this identity.");
        }

        Ok(())
    }
    
    pub fn get_wallets(ctx: Context<LinkedWallets>, identity: Pubkey) -> Result<Vec<Pubkey>> {
        let factory_account = &ctx.accounts.factory;

        let wallets: Vec<Pubkey> = factory_account
            .linked_wallets
            .iter()
            .cloned()
            .filter(|&linked_wallet| factory_account.wallet_to_identity.iter().any(|&(w, i)| w == linked_wallet && i == identity))
            .collect();

        Ok(wallets)
    }

    pub fn get_identity(ctx: Context<LinkedWallets>, wallet: Pubkey) -> Result<Pubkey> {
        if let Some((_, identity)) = ctx.accounts.factory.wallet_to_identity.iter().find(|&&(w, _)| w == wallet) {
            Ok(*identity)
        } else {
            Err(ErrorCode::WalletNotLinked.into())
        }
    }

    pub fn get_owner(ctx: Context<LinkedWallets>) -> Result<Pubkey> {
        Ok(ctx.accounts.factory.owner)
    }

    pub fn set_owner(ctx: Context<LinkedWallets>, new_owner: Pubkey) -> Result<()> {
        let factory_account = &mut ctx.accounts.factory;

        require!(ctx.accounts.signer.key == &factory_account.owner, ErrorCode::Unauthorized);

        factory_account.owner = new_owner;
        msg!("New owner set: {:?}", new_owner);
        emit!(OwnerSet {
            new_owner,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = Factory::calc_size(100, 100))]
    pub factory: Account<'info, Factory>,
    #[account(mut, signer)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateIdentity<'info> {
    #[account(mut)]
    pub factory: Account<'info, Factory>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(address = identity_lib::ID)]
    pub identity_program: Program<'info, Identity>,
}

#[derive(Accounts)]
pub struct LinkedWallets<'info> {
    #[account(mut)]
    pub factory: Account<'info, Factory>,
    #[account(signer)]
    pub signer: Signer<'info>, // Ensures global owner in Factory is used for authorization
}

#[derive(Accounts)]
pub struct GetIdentity<'info> {
    pub factory: Account<'info, Factory>,
}

#[account]
#[derive(Default)]
pub struct Factory {
    pub initialized: bool,
    pub owner: Pubkey,
    pub identity_addresses: Vec<Pubkey>,
    pub linked_wallets: Vec<Pubkey>,
    pub wallet_to_identity: Vec<(Pubkey, Pubkey)>, // Vec for wallet to identity
}

impl Factory {
    pub fn calc_size(id_count: usize, wallet_count: usize) -> usize {
        8 + 1 + 32 + 4 + id_count * 32 + 4 + wallet_count * 32
    }
}

#[event]
pub struct FactoryInitialized {
    pub owner: Pubkey,
}

#[event]
pub struct IdentityCreated {
    pub wallet: Pubkey,
    pub identity_address: Pubkey,
}

#[event]
pub struct WalletLinked {
    pub wallet: Pubkey,
    pub identity: Pubkey,
}

#[event]
pub struct WalletUnlinked {
    pub wallet: Pubkey,
    pub identity: Pubkey,
}

#[event]
pub struct OwnerSet {
    pub new_owner: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The contract has already been initialized.")]
    AlreadyInitialized,
    #[msg("Unauthorized operation attempted.")]
    Unauthorized,
    #[msg("The wallet address is not linked to any identity.")]
    WalletNotLinked,
}