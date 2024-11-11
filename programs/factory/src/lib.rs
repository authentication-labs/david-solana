use std::io::Read;
use byteorder::{ReadBytesExt, LittleEndian};


use anchor_lang::{prelude::*, solana_program, Result, require};
use solana_program::{instruction::{Instruction, AccountMeta}, program::invoke_signed, pubkey::Pubkey};
use identity_lib::program::Identity;
use oapp::LzReceiveParams;

pub mod errors;
pub mod state;
pub mod instructions;

use errors::*;
use crate::instructions::*;

pub const LZ_RECEIVE_TYPES_SEED: &[u8] = oapp::LZ_RECEIVE_TYPES_SEED;

pub const MAX_FEE_BASIS_POINTS: u16 = 10_000;

declare_id!("CyKce9sNf2SHyLZgS9URiu2o1tDs8UeASzpwtH3dpadt");

#[program]
pub mod factory_contract {
    use super::*;

    pub fn set_remote(mut ctx: Context<SetRemote>, params: SetRemoteParams) -> Result<()> {
        SetRemote::apply(&mut ctx, &params)
    }

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
        ctx: Context<LzReceive>,
        wallet: Pubkey,
        salt: [u8; 32],
        initial_management_key: Pubkey,
    ) -> Result<()> {
        require!(*ctx.accounts.payer.key == ctx.accounts.factory.owner, ErrorCode::Unauthorized);
    
        let seeds = &[b"identity", wallet.as_ref(), salt.as_ref()];
        let (identity_address, bump) = Pubkey::find_program_address(seeds, ctx.program_id);
    
        let mut data = initial_management_key.to_bytes().to_vec();
        data.extend(ctx.accounts.factory.key().to_bytes());
    
        let instruction = Instruction {
            program_id: ctx.accounts.identity_program.key(),
            accounts: vec![
                AccountMeta::new(identity_address, false),
                AccountMeta::new(*ctx.accounts.payer.key, true),
                AccountMeta::new(ctx.accounts.factory.key(), false),
                AccountMeta::new_readonly(*ctx.accounts.system_program.key, false),
            ],
            data,
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
    pub fn lz_receive(ctx: Context<LzReceive>, params: LzReceiveParams) -> Result<()> {
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


fn add_claim(
    ctx: Context<LzReceive>,
    wallet: Pubkey,
    topic: u64,
    scheme: u64,
    issuer_wallet: Pubkey,
    signature: [u8; 64],
    data: Vec<u8>,
    uri: String,
) -> Result<()> {
    let identity_address = find_identity_address(&ctx, wallet)?;

    let issuer = find_identity_address(&ctx, wallet)?;

    let instruction = Instruction {
        program_id: issuer, // Assuming you still want to instruct on the identity program
        accounts: vec![
            AccountMeta::new(identity_address, false),
            AccountMeta::new(ctx.accounts.factory.owner, true),
            AccountMeta::new(ctx.accounts.claims_account.key(), false),
        ],
        data: {
            let mut data_vec = vec![]; 
            data_vec.extend_from_slice(&topic.to_le_bytes());
            data_vec.extend_from_slice(&scheme.to_le_bytes());
            data_vec.extend_from_slice(&issuer_wallet.to_bytes());
            data_vec.extend_from_slice(&issuer.to_bytes()); // Use identity contract as issuer
            data_vec.extend_from_slice(&signature);
            data_vec.extend_from_slice(&(data.len() as u32).to_le_bytes());
            data_vec.extend_from_slice(&data); 
            data_vec.extend_from_slice(&(uri.len() as u32).to_le_bytes());
            data_vec.extend_from_slice(uri.as_bytes());
            data_vec
        },
    };

    invoke_signed(
        &instruction,
        &[
            ctx.accounts.factory.to_account_info(),
            ctx.accounts.identity_program.to_account_info(),
            ctx.accounts.claims_account.to_account_info(),
        ],
        &[],
    )?;

    msg!("Claim added to identity for wallet: {:?}", wallet);
    emit!(ClaimAddedEvent { wallet, topic, scheme, issuer_wallet, issuer, uri });

    Ok(())
}


fn remove_claim(
    ctx: Context<LzReceive>,
    wallet: Pubkey,
    topic: u64,
) -> Result<()> {
    let identity_address = find_identity_address(&ctx, wallet)?;

    let instruction = Instruction {
        program_id: ctx.accounts.identity_program.key(),
        accounts: vec![
            AccountMeta::new(identity_address, false),
            AccountMeta::new(ctx.accounts.factory.owner, true),
            AccountMeta::new(ctx.accounts.claims_account.key(), false),
        ],
        data: {
            let mut data = vec![];
            data.extend_from_slice(&topic.to_le_bytes());
            data.extend_from_slice(&identity_address.to_bytes());
            data
        },
    };


    invoke_signed(
        &instruction,
        &[
            ctx.accounts.factory.to_account_info(),
            ctx.accounts.identity_program.to_account_info(),
            ctx.accounts.claims_account.to_account_info(),
        ],
        &[],
    )?;

    msg!("Claim removed from identity for wallet: {:?}", wallet);
    emit!(ClaimRemovedEvent { wallet, topic });

    Ok(())
}

fn add_key(
    ctx: Context<LzReceive>,
    wallet: Pubkey,
    key: Pubkey,
    purpose: u32,
    key_type: u32,
) -> Result<()> {
    let identity_address = find_identity_address(&ctx, wallet)?;

    let instruction = Instruction {
        program_id: ctx.accounts.identity_program.key(),
        accounts: vec![
            AccountMeta::new(identity_address, false),
            AccountMeta::new(ctx.accounts.factory.owner, true),
            AccountMeta::new(ctx.accounts.keys_account.key(), false),
        ],
        data: {
            let mut data = vec![];
            data.extend_from_slice(&key.to_bytes());
            data.extend_from_slice(&purpose.to_le_bytes());
            data.extend_from_slice(&key_type.to_le_bytes());
            data
        },
    };

    invoke_signed(
        &instruction,
        &[
            ctx.accounts.factory.to_account_info(),
            ctx.accounts.identity_program.to_account_info(),
            ctx.accounts.keys_account.to_account_info(),
        ],
        &[],
    )?;

    msg!("Key added to identity for wallet: {:?}", wallet);
    emit!(KeyAddedEvent { wallet, key, purpose, key_type });

    Ok(())
}

fn remove_key(
    ctx: Context<LzReceive>,
    wallet: Pubkey,
    key: Pubkey,
    purpose: u32,
) -> Result<()> {
    let identity_address = find_identity_address(&ctx, wallet)?;

    let instruction = Instruction {
        program_id: ctx.accounts.identity_program.key(),
        accounts: vec![
            AccountMeta::new(identity_address, false),
            AccountMeta::new(ctx.accounts.factory.owner, true),
            AccountMeta::new(ctx.accounts.keys_account.key(), false),
        ],
        data: {
            let mut data = vec![];
            data.extend_from_slice(&key.to_bytes());
            data.extend_from_slice(&purpose.to_le_bytes());
            data
        },
    };

    invoke_signed(
        &instruction,
        &[
            ctx.accounts.factory.to_account_info(),
            ctx.accounts.identity_program.to_account_info(),
            ctx.accounts.keys_account.to_account_info(),
        ],
        &[],
    )?;

    msg!("Key removed from identity for wallet: {:?}", wallet);
    emit!(KeyRemovedEvent { wallet, key, purpose });

    Ok(())
}

fn find_identity_address(ctx: &Context<LzReceive>, wallet: Pubkey) -> Result<Pubkey> {
    ctx.accounts.factory
        .wallet_to_identity
        .iter()
        .find(|&&(w, _)| w == wallet)
        .map(|&(_, identity)| identity)
        .ok_or(ErrorCode::WalletNotLinked.into())
}

fn decode_add_key_payload(payload: &[u8]) -> anchor_lang::Result<(Pubkey, Pubkey, u32, u32)> {
    let mut cursor = std::io::Cursor::new(payload);

    let mut wallet_bytes = [0u8; 32];
    cursor.read_exact(&mut wallet_bytes)?;
    let wallet = Pubkey::new_from_array(wallet_bytes);

    let mut key_bytes = [0u8; 32];
    cursor.read_exact(&mut key_bytes)?;
    let key = Pubkey::new_from_array(key_bytes);

    let purpose = cursor.read_u32::<LittleEndian>()?;
    let key_type = cursor.read_u32::<LittleEndian>()?;

    Ok((wallet, key, purpose, key_type))
}

fn decode_remove_key_payload(payload: &[u8]) -> anchor_lang::Result<(Pubkey, Pubkey, u32)> {
    let mut cursor = std::io::Cursor::new(payload);

    let mut wallet_bytes = [0u8; 32];
    cursor.read_exact(&mut wallet_bytes)?;
    let wallet = Pubkey::new_from_array(wallet_bytes);

    let mut key_bytes = [0u8; 32];
    cursor.read_exact(&mut key_bytes)?;
    let key = Pubkey::new_from_array(key_bytes);

    let purpose = cursor.read_u32::<LittleEndian>()?;

    Ok((wallet, key, purpose))
}

fn decode_message(message: &[u8]) -> Result<(String, Vec<u8>)> {
    let method_name_length = message[0] as usize;
    let method_name = &message[1..1 + method_name_length];
    let payload = &message[1 + method_name_length..];
    
    let method_name_str = String::from_utf8(method_name.to_vec()).map_err(|_| ProgramError::InvalidInstructionData)?;
    Ok((method_name_str, payload.to_vec()))
}

fn decode_create_identity_payload(payload: &[u8]) -> std::result::Result<(Pubkey, [u8; 32]), ProgramError> {
    let mut cursor = std::io::Cursor::new(payload);

    let mut identity_owner_bytes = [0u8; 32];
    cursor.read_exact(&mut identity_owner_bytes)?;
    let identity_owner = Pubkey::new_from_array(identity_owner_bytes);

    let mut salt_bytes = [0u8; 32];
    cursor.read_exact(&mut salt_bytes)?;

    Ok((identity_owner, salt_bytes))
}

fn decode_add_claim_payload(
    payload: &[u8],
) -> std::result::Result<(Pubkey, u64, u64, Pubkey, [u8; 64], Vec<u8>, String), ProgramError> {
    let mut cursor = std::io::Cursor::new(payload);

    // Decode `wallet`
    let mut wallet_bytes = [0u8; 32];
    cursor.read_exact(&mut wallet_bytes)?;
    let wallet = Pubkey::new_from_array(wallet_bytes);

    // Decode the topic and scheme
    let topic = cursor.read_u64::<LittleEndian>()?;
    let scheme = cursor.read_u64::<LittleEndian>()?;

    // Issuer Wallet and Issuer use `wallet`
    let issuer_wallet = wallet;

    // Decode the remaining fields; signature, data, and uri
    let mut signature = [0u8; 64];
    cursor.read_exact(&mut signature)?;

    let data_size = cursor.read_u32::<LittleEndian>()? as usize;
    let mut data = vec![0u8; data_size];
    cursor.read_exact(&mut data)?;

    let mut uri_buffer = vec![];
    cursor.read_to_end(&mut uri_buffer)?;
    let uri = String::from_utf8(uri_buffer).map_err(|_| ProgramError::InvalidInstructionData)?;

    Ok((wallet, topic, scheme, issuer_wallet, signature, data, uri))
}

fn decode_remove_claim_payload(
    payload: &[u8],
) -> anchor_lang::Result<(Pubkey, u64)> {
    let mut cursor = std::io::Cursor::new(payload);

    let mut wallet_bytes = [0u8; 32];
    cursor.read_exact(&mut wallet_bytes)?;
    let wallet = Pubkey::new_from_array(wallet_bytes);

    let topic = cursor.read_u64::<LittleEndian>()?;

    let mut issuer_bytes = [0u8; 32];
    cursor.read_exact(&mut issuer_bytes)?;

    Ok((wallet, topic))
}

#[derive(Accounts)]
pub struct AddKey<'info> {
    #[account(mut)]
    pub factory: Account<'info, Factory>,
    #[account(mut)]
    pub keys_account: Account<'info, KeysAccount>,
    pub identity_program: Program<'info, Identity>,
}

#[account]
pub struct KeysAccount {
    pub keys: Vec<NewKey>,
}

#[derive(Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize, Debug)]
pub enum KeyPurpose {
    Management = 1,
    Action = 2,
    Claim = 3,
    Encryption = 4,
}

impl TryFrom<u32> for KeyPurpose {
    type Error = Error; 

    fn try_from(value: u32) -> std::result::Result<KeyPurpose, Error> {
        match value {
            1 => Ok(KeyPurpose::Management),
            2 => Ok(KeyPurpose::Action),
            3 => Ok(KeyPurpose::Claim),
            4 => Ok(KeyPurpose::Encryption),
            _ => Err(ErrorCode::InvalidKeyPurpose.into()),
        }
    }
}

#[derive(Clone, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum KeyType {
    ECDSA = 1,
    RSA = 2,
}

#[account]
pub struct NewKey {
    pub purposes: Vec<KeyPurpose>,
    pub key_type: KeyType,
    pub key: [u8; 32],
}

impl TryFrom<u32> for KeyType {
    type Error = Error;

    fn try_from(value: u32) -> std::result::Result<KeyType, Error> {
        match value {
            1 => Ok(KeyType::ECDSA),
            2 => Ok(KeyType::RSA),
            _ => Err(ErrorCode::InvalidKeyType.into()),
        }
    }
}

#[account]
pub struct ClaimsAccount {
    pub claims: Vec<([u8; 32], Claim)>, 
}

#[account]
pub struct Claim {
    topic: u64,          
    scheme: u64,
    issuer_wallet: Pubkey,
    issuer: Pubkey,
    signature: [u8; 64], 
    data: Vec<u8>,      
    uri: String,
}

#[derive(Accounts)]
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
    pub signer: Signer<'info>,
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
    pub wallet_to_identity: Vec<(Pubkey, Pubkey)>,
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

#[event]
pub struct KeyAddedEvent {
    pub wallet: Pubkey,
    pub key: Pubkey,
    pub purpose: u32,
    pub key_type: u32,
}

#[event]
pub struct KeyRemovedEvent {
    pub wallet: Pubkey,
    pub key: Pubkey,
    pub purpose: u32,
}

#[event]
pub struct ClaimAddedEvent {
    pub wallet: Pubkey,
    pub topic: u64,
    pub scheme: u64,
    pub issuer_wallet: Pubkey,
    pub issuer: Pubkey,
    pub uri: String,
}

#[event]
pub struct ClaimRemovedEvent {
    pub wallet: Pubkey,
    pub topic: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The contract has already been initialized.")]
    AlreadyInitialized,
    #[msg("Unauthorized operation attempted.")]
    Unauthorized,
    #[msg("The wallet address is not linked to any identity.")]
    WalletNotLinked,
    #[msg("Invalid key purpose provided.")]
    InvalidKeyPurpose,
    #[msg("The specified key type is invalid.")]
    InvalidKeyType,
}