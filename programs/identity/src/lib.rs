use anchor_lang::{prelude::*, solana_program::hash::hash, Key as AnchorKey,};

declare_id!("8DXtpG31GL4L215EeREcPhQCFgFjWcWQjX27d9XEFsRo");

#[program]
pub mod identity {
    use super::*;

    pub fn get_initialized(ctx: Context<Initialize>) -> Result<bool> {
        Ok(ctx.accounts.identity_account.initialized)
    }

    pub fn initialize(ctx: Context<Initialize>, initial_management_key: Pubkey) -> Result<()> {
        let identity_account = &mut ctx.accounts.identity_account;
        let keys = &mut ctx.accounts.keys;
        let key_account = &mut ctx.accounts.key_account;

        if identity_account.initialized {
            return Err(Error::AlreadyInitialized.into());
        }

        let key_hash = hash_key(&initial_management_key);
        
        key_account.purposes = vec![KeyPurpose::Management];
        key_account.key_type = KeyType::ECDSA;
        key_account.key = key_hash;

        // Create a Key instance for the keys vector
        let new_key = Key {
            purposes: key_account.purposes.clone(),
            key_type: key_account.key_type,
            key: key_account.key,
        };

        keys.keys.push(new_key);
        identity_account.initialized = true;

        msg!("Initialized from");
        Ok(())
    }

    // pub fn get_key(_ctx: Context<>, key: Pubkey) -> Result<Key> {
    //     let key_hash = hash_key(&key);


    //     Ok(())
    // }

    pub fn add_key(ctx: Context<AddKey>, manager: Pubkey, key: Pubkey, purpose: u32, key_type: u32 ) -> Result<()> {
        identity_require_auth( &ctx, &manager, KeyPurpose::Management)?;

        
        let key_hash = hash_key( &key);
        let key_purpose = KeyPurpose::try_from(purpose).map_err(|_| anchor_lang::error::Error::from(Error::InvalidKeyPurpose))?;
        let key_type_enum = KeyType::try_from(key_type).map_err(|_| anchor_lang::error::Error::from(Error::InvalidKeyType))?;

        let keys = &mut ctx.accounts.keys;
        let mut key_found = false;
        
        for i in 0..keys.keys.len() {
            let k = &mut keys.keys[i];
            if k.key == key_hash {
                if k.purposes.contains(&key_purpose) {
                    return Err(Error::KeyConflict.into());
                } else {
                    k.purposes.push(key_purpose);
                    key_found = true;
                    break;
                }
            }
        }

        if !key_found {
            let new_key = Key {
                purposes: vec![key_purpose],
                key_type: key_type_enum,
                key: key_hash.clone(),
            };
            keys.keys.push(new_key);
        }
        Ok(())
    }

    pub fn add_claim(_ctx: Context<AddClaim>) -> Result<()> {
        Ok(())
    }

    pub fn is_claim_valid(_ctx: Context<IsClaimValid>) -> Result<()> {
        Ok(())
    }
}

fn identity_require_auth(ctx: &Context<AddKey>, sender: &Pubkey, key_type: KeyPurpose) -> Result<()> {
    let key_hash = hash_key(&sender);

    if !key_has_purpose(&ctx, &key_hash, key_type) {
        return Err(Error::InsufficientPermissions.into());
    }

    Ok(())
}

fn key_has_purpose(ctx: &Context<AddKey>, key_hash: &[u8; 32], purpose: KeyPurpose) -> bool {
    let keys_account = &ctx.accounts.keys;
    keys_account.keys.iter().any(|k| k.key == *key_hash && k.purposes.contains(&purpose))
}

fn hash_key(key: &Pubkey) -> [u8; 32] {
    let key_bytes = key.to_bytes();
    let hashed = hash(&key_bytes);

    hashed.to_bytes()
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 1)] // 8 bytes for discriminator, 1 for bool
    pub identity_account: Account<'info, IdentityAccount>,
    #[account(init, payer = user, space = Key::LEN + 8)] // Add 8 for discriminator
    pub key_account: Account<'info, Key>,
    pub keys: Account<'info, KeysAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddKey<'info> {
    #[account(mut)]
    pub keys: Account<'info, KeysAccount>,
}

#[derive(Accounts)]
pub struct AddClaim {}

#[derive(Accounts)]
pub struct IsClaimValid {}

#[account]
pub struct IdentityAccount {
    pub initialized: bool,
}

#[derive(Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize, Debug)]
pub enum KeyPurpose {
    Management = 1,
    Action = 2,
    Claim = 3,
    Encryption = 4,
}

impl TryFrom<u32> for KeyPurpose {
    type Error = Error; // Ensure your error type matches the intended return

    fn try_from(value: u32) -> std::result::Result<KeyPurpose, Error> {
        match value {
            1 => Ok(KeyPurpose::Management),
            2 => Ok(KeyPurpose::Action),
            3 => Ok(KeyPurpose::Claim),
            4 => Ok(KeyPurpose::Encryption),
            _ => Err(Error::InvalidKeyPurpose),
        }
    }
}

#[derive(Clone, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum KeyType {
    ECDSA = 1,
    RSA = 2,
}

#[account]
pub struct Key {
    pub purposes: Vec<KeyPurpose>,
    pub key_type: KeyType,
    pub key: [u8; 32],
}

impl Key {
    const LEN: usize = 4 + (1 * 4) + 4 + 32;
}

impl TryFrom<u32> for KeyType {
    type Error = Error;

    fn try_from(value: u32) -> std::result::Result<KeyType, Error> {
        match value {
            1 => Ok(KeyType::ECDSA),
            2 => Ok(KeyType::RSA),
            _ => Err(Error::InvalidKeyType),
        }
    }
}
#[account]
pub struct KeysAccount {
    // We store the keys as a vector within this account
    pub keys: Vec<Key>,
}

#[account]
pub struct ClaimAccount {
    claim_topic: u32,
    scheme: u32,
    issuer: Pubkey,
    signature: [u8; 64],
    data: Vec<u8>,
    uri: String,
}

#[error_code]
pub enum Error {
    #[msg("The item is already initialized.")]
    AlreadyInitialized = 1,
    #[msg("The specified key was not found.")]
    KeyNotFound,
    #[msg("Invalid key purpose provided.")]
    InvalidKeyPurpose,
    #[msg("The specified key type is invalid.")]
    InvalidKeyType,
    #[msg("There is a conflict with an existing key.")]
    KeyConflict,
    #[msg("The index provided is out of bounds.")]
    IndexOutOfBounds,
    #[msg("The specified claim could not be found.")]
    ClaimNotFound,
    #[msg("The key does not have the required purpose.")]
    KeyDoesNotHavePurpose,
    #[msg("The claim has already been revoked.")]
    ClaimAlreadyRevoked,
    #[msg("Insufficient permissions for this action.")]
    InsufficientPermissions,
    #[msg("The signature is invalid.")]
    InvalidSignature,
    #[msg("The claim is invalid.")]
    InvalidClaim,
    #[msg("The issuer is invalid.")]
    InvalidIssuer,
    #[msg("The address bytes are invalid.")]
    InvalidAddressBytes,
}