use anchor_lang::{prelude::*, solana_program::hash::hash, Key as AnchorKey,};
use crate::utils::verify_ed25519_ix;
use solana_program::sysvar::instructions::{load_instruction_at_checked, ID as IX_ID};
use solana_program::ed25519_program::ID as ED25519_ID;
use solana_program::instruction::Instruction;
pub mod utils;
pub mod error;

declare_id!("3BWQvZ8vUQewDzAcDmoYa2PzKFazgaCBnKQNFQx15hXM");

#[program]
pub mod claim_issuer {
    pub use super::*;

    pub fn get_initialized(_ctx: Context<Initialize>) -> Result<bool> {
        Ok(_ctx.accounts.identity_account.initialized)
    }

    pub fn initialize(_ctx: Context<Initialize>, initial_management_key: Pubkey) -> Result<()> {
        let identity_account = &mut _ctx.accounts.identity_account;
        let keys = &mut _ctx.accounts.keys;
        let key_account = &mut _ctx.accounts.key_account;

        if identity_account.initialized {
            return Err(Error::AlreadyInitialized.into());
        }

        let key_hash = hash_key(&initial_management_key);
        
        key_account.purposes = vec![KeyPurpose::Management];
        key_account.key_type = KeyType::ECDSA;
        key_account.key = key_hash;

        // Create a Key instance for the keys vector
        let new_key = NewKey {
            purposes: key_account.purposes.clone(),
            key_type: key_account.key_type,
            key: key_account.key,
        };

        keys.keys.push(new_key);
        identity_account.initialized = true;
        Ok(())
    }

    pub fn get_key(_ctx: Context< _Key>, key: Pubkey) -> Result<NewKey> {        
        let key_hash = hash_key(&key);
        let keys_account = &_ctx.accounts.keys;
        keys_account.keys.iter()
            .find(|k| k.key == key_hash)
            .cloned()
            .ok_or(Error::KeyNotFound.into())
    }

    pub fn get_keys(_ctx: Context<_Key>) -> Result<Vec<NewKey>> {
        Ok(_ctx.accounts.keys.keys.clone())   
    }

    pub fn add_key(_ctx: Context<_Key>, manager: Pubkey, key: Pubkey, purpose: u32, key_type: u32 ) -> Result<()> {
        identity_require_auth(&_ctx.accounts.keys, &manager, KeyPurpose::Management)?;
        
        let key_hash = hash_key( &key);
        let key_purpose = KeyPurpose::try_from(purpose).map_err(|_| anchor_lang::error::Error::from(Error::InvalidKeyPurpose))?;
        let key_type_enum = KeyType::try_from(key_type).map_err(|_| anchor_lang::error::Error::from(Error::InvalidKeyType))?;

        let keys = &mut _ctx.accounts.keys;
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
            let new_key = NewKey {
                purposes: vec![key_purpose],
                key_type: key_type_enum,
                key: key_hash.clone(),
            };
            keys.keys.push(new_key);
        }

        emit!(KeyAdded {
            manager,
            key,
            purpose,
            key_type,
        });
        
        Ok(())
    }

    pub fn remove_key<'info>(_ctx: Context< _Key>, manager: Pubkey, key: Pubkey, purpose: u32) -> Result<()> {
        identity_require_auth(&_ctx.accounts.keys, &manager, KeyPurpose::Management)?;

        let key_hash = hash_key( &key);
        let key_purpose = KeyPurpose::try_from(purpose).map_err(|_| Error::InvalidKeyPurpose)?;
    
        let keys_account = &mut _ctx.accounts.keys;
    
        if !keys_account.keys.iter().any(|k| k.key == key_hash) {
            return Err(Error::KeyNotFound.into());
        }
    
        for i in 0..keys_account.keys.len() {
            if keys_account.keys[i].key == key_hash {
                if let Some(pos) = keys_account.keys[i].purposes.iter().position(|&p| p == key_purpose) {
                    keys_account.keys[i].purposes.remove(pos);
                    
                    if keys_account.keys[i].purposes.is_empty() {
                        keys_account.keys.remove(i);
                    }
                } else {
                    return Err(Error::KeyDoesNotHavePurpose.into());
                }
                break;
            }
        }

        emit!(KeyRemoved {
            manager,
            key,
            purpose,
        });
        Ok(())
    }

    pub fn add_claim(
        ctx: Context<ClaimContext>,
        sender: Pubkey, 
        topic: u64,
        scheme: u64,
        issuer_wallet: Pubkey,
        issuer: Pubkey,
        signature: [u8; 64],
        data: Vec<u8>,
        uri: String,
    ) -> Result<[u8; 32]> {
        identity_require_auth(&ctx.accounts.key_context.keys, &sender, KeyPurpose::Claim)?;
        let current_program_id = ctx.program_id;
        if *current_program_id != issuer {
            let is_valid = is_claim_valid(
                &ctx.accounts.instructions,
                &ctx.accounts.key_context.keys,
                issuer_wallet,
                *current_program_id,
                topic,
                signature,
                data.clone(),
            )?;
            if !is_valid {
                return Err(Error::InvalidClaim.into());
            }
        }
    
        
        let claim_id = hash_claim(&issuer, topic);
        let claims_account = &mut ctx.accounts.claims_account;
        let new_claim = Claim {
            topic,
            scheme,
            issuer_wallet,
            issuer,
            signature,
            data: data.clone(),
            uri: uri.clone(),
        };
        claims_account.claims.push((claim_id, new_claim));
    
        // Emit event for added claim
        emit!(ClaimAdded {
            sender,
            claim_id,
            topic,
            scheme,
            issuer,
            issuer_wallet,
            signature,
            data,
            uri,
        });
    
        Ok(claim_id)
    }


    pub fn remove_claim(ctx: Context<ClaimContext>, sender: Pubkey, claim_id: [u8; 32]) -> Result<()> {
        // Ensure the action is authorized
        identity_require_auth(&ctx.accounts.key_context.keys, &sender, KeyPurpose::Claim)?;

        // Locate the claim
        let claims_account = &mut ctx.accounts.claims_account;

        // Find the index of the claim
        let pos = claims_account.claims.iter().position(|(id, _)| *id == claim_id)
            .ok_or(Error::ClaimNotFound)?;

        // Remove the claim from the list
        claims_account.claims.remove(pos);

        // Emit an event for removed claim
        emit!(ClaimRemoved {
            sender,
            claim_id,
        });

        Ok(())
    }

    pub fn get_claim_by_id(_ctx: Context<ClaimContext>, claim_id: [u8; 32]) -> Result<Option<Claim>> {
        let claims_account = &_ctx.accounts.claims_account;
        let claim = claims_account.claims.iter()
            .find(|(id, _)| *id == claim_id)
            .map(|(_, claim)| claim.clone());
        Ok(claim)
    }


    pub fn revoke_claim(
        ctx: Context<ClaimContext>,
        sender: Pubkey,
        claim_id: [u8; 32],
    ) -> Result<()> {
        // Ensure the action is authorized
        identity_require_auth(&ctx.accounts.key_context.keys, &sender, KeyPurpose::Management)?;
    
        // Locate the claim
        let claims_account = &mut ctx.accounts.claims_account;
    
        // Find the claim
        let claim = claims_account.claims.iter()
            .find(|(id, _)| *id == claim_id)
            .ok_or(Error::ClaimNotFound)?;
    
        // Check if the claim is already revoked
        let revoked_claims = &mut ctx.accounts.revoked_claims;
        if revoked_claims.revoked_claims.iter().any(|&s| s == claim.1.signature) {
            return Err(Error::ClaimAlreadyRevoked.into());
        }
    
        // Add the claim's signature to the revoked claims
        revoked_claims.revoked_claims.push(claim.1.signature.clone());
    
        // Emit an event for revoked claim
        emit!(ClaimRevoked {
            sender,
            claim_id,
        });
    
        Ok(())
    }

    
    pub fn is_claim_revoked(
        ctx: Context<ClaimContext>,
        signature: [u8; 64],
    ) -> Result<bool> {
        let revoked_claims = &ctx.accounts.revoked_claims;
        Ok(revoked_claims.revoked_claims.iter().any(|&s| s == signature))
    }

}

#[event]
pub struct KeyAdded {
    pub manager: Pubkey,
    pub key: Pubkey,
    pub purpose: u32,
    pub key_type: u32,
}

#[event]
pub struct KeyRemoved {
    pub manager: Pubkey,
    pub key: Pubkey,
    pub purpose: u32,
}

#[event]
pub struct ClaimAdded {
    pub sender: Pubkey,
    pub claim_id: [u8; 32],
    pub topic: u64,
    pub scheme: u64,
    pub issuer: Pubkey,
    pub issuer_wallet: Pubkey,
    pub signature: [u8; 64],
    pub data: Vec<u8>,
    pub uri: String,
}

#[event]
pub struct ClaimRemoved {
    pub sender: Pubkey,
    pub claim_id: [u8; 32],
}

#[event]
pub struct ClaimRevoked {
    pub sender: Pubkey,
    pub claim_id: [u8; 32],
}

#[account]
pub struct RevokedClaimsAccount {
    pub revoked_claims: Vec<[u8; 64]>,
}

fn identity_require_auth(keys_account: &Account<KeysAccount>, sender: &Pubkey, key_type: KeyPurpose) -> Result<()> {
    let key_hash = hash_key(sender);

    if !key_has_purpose(keys_account, &key_hash, key_type) {
        return Err(Error::InsufficientPermissions.into());
    }

    Ok(())
}

fn key_has_purpose(keys_account: &Account<KeysAccount>, key_hash: &[u8; 32], purpose: KeyPurpose) -> bool {
    keys_account.keys.iter().any(|k| k.key == *key_hash && k.purposes.contains(&purpose))
}
fn hash_key(key: &Pubkey) -> [u8; 32] {
    let key_bytes = key.to_bytes();
    let hashed = hash(&key_bytes);

    hashed.to_bytes()
}

pub fn hash_claim(issuer: &Pubkey, topic: u64) -> [u8; 32] {
    let issuer_bytes = issuer.to_bytes();

    let topic_bytes = topic.to_le_bytes();

    let mut concatenated_bytes = Vec::new();
    concatenated_bytes.extend_from_slice(&issuer_bytes);
    concatenated_bytes.extend_from_slice(&topic_bytes);

    let hashed = hash(&concatenated_bytes);

    hashed.to_bytes()
}


pub fn is_claim_valid(
    instructions: &AccountInfo,
    keys: &Account<KeysAccount>,
    issuer_wallet: Pubkey,
    identity: Pubkey,
    topic: u64,
    signature: [u8; 64],
    data: Vec<u8>,
) -> Result<bool> {
    // Concatenate data
    let mut concatenated_bytes = Vec::new();
    concatenated_bytes.extend_from_slice(&identity.to_bytes());
    concatenated_bytes.extend_from_slice(&topic.to_le_bytes());
    concatenated_bytes.extend_from_slice(&data);

    // Verify the Ed25519 signature
    let instruction_sysvar_info = instructions.to_account_info();
    let ix: Instruction = load_instruction_at_checked(0, &instruction_sysvar_info)?;

    // Check that the instruction is an Ed25519Program instruction
    if ix.program_id != ED25519_ID {
        return Err(Error::InvalidSignature.into());
    }

    // Verify the Ed25519Program instruction fields
    verify_ed25519_ix(&ix, &issuer_wallet.to_bytes(), &concatenated_bytes, &signature)?;

    // Check if the issuer's key has the Claim purpose
    let issuer_wallet_hash = hash_key(&issuer_wallet);
    if key_has_purpose(keys, &issuer_wallet_hash, KeyPurpose::Claim) {
        Ok(true)
    } else {
        Err(Error::InvalidClaim.into())
    }
}


/// External instruction that only gets executed if
/// an `Ed25519Program.createInstructionWithPublicKey`
/// instruction was sent in the same transaction.
pub fn verify_ed25519(ctx: Context<Verify>, pubkey: [u8; 32], msg: Vec<u8>, sig: [u8; 64]) -> Result<()> {
    // Get what should be the Ed25519Program instruction
    let ix: Instruction = load_instruction_at_checked(0, &ctx.accounts.ix_sysvar).map_err(|e| anchor_lang::prelude::ProgramError::from(e))?;

    // Check that ix is what we expect to have been sent
    utils::verify_ed25519_ix(&ix, &pubkey, &msg, &sig)?;

    // Do other stuff
    
    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 1)] // 8 bytes for discriminator, 1 for bool
    pub identity_account: Account<'info, IdentityAccount>,
    #[account(init, payer = user, space = NewKey::LEN + 1)] // 8 bytes for discriminator, 1 for bool
    pub key_account: Account<'info, NewKey>,
    pub keys: Account<'info, KeysAccount>,
    #[account(init, payer = user, space = Claim::LEN + 8)]
    pub claims_account: Account<'info, ClaimsAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct _Key<'info> {
    #[account(mut)]
    pub keys: Account<'info, KeysAccount>,
}

#[derive(Accounts)]
pub struct ClaimContext<'info> {
    #[account(mut)]
    pub claims_account: Account<'info, ClaimsAccount>,
    pub key_context: _Key<'info>,
    /// CHECK: This is safe because we are only reading the instructions sysvar.
    #[account(address = solana_program::sysvar::instructions::id())]
    pub instructions: AccountInfo<'info>,
    #[account(mut)]
    pub revoked_claims: Account<'info, RevokedClaimsAccount>,
}
/// Context accounts
#[derive(Accounts)]
pub struct Verify<'info> {
    pub sender: Signer<'info>,

    /// CHECK: The address check is needed because otherwise
    /// the supplied Sysvar could be anything else.
    /// The Instruction Sysvar has not been implemented
    /// in the Anchor framework yet, so this is the safe approach.
    #[account(address = IX_ID)]
    pub ix_sysvar: AccountInfo<'info>,
}

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
pub struct NewKey {
    pub purposes: Vec<KeyPurpose>,
    pub key_type: KeyType,
    pub key: [u8; 32],
}

impl NewKey {
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
    pub keys: Vec<NewKey>,
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

impl Claim {
    const LEN: usize = 8 + 8 + 32 + 32 + 64 + 2048 + 512;
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