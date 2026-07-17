use crate::error::{Result, VaultError};
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Argon2id parameters. m_cost is in KiB. 64 MiB / t=3 / p=1 per the design plan.
pub const DEFAULT_M_COST: u32 = 65536;
pub const DEFAULT_T_COST: u32 = 3;
pub const DEFAULT_P_COST: u32 = 1;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultMetaFile {
    pub version: u32,
    /// hex-encoded 16-byte salt
    pub salt: String,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl VaultMetaFile {
    pub fn new_random() -> Self {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        VaultMetaFile {
            version: 1,
            salt: hex::encode(salt),
            m_cost: DEFAULT_M_COST,
            t_cost: DEFAULT_T_COST,
            p_cost: DEFAULT_P_COST,
        }
    }

    pub fn salt_bytes(&self) -> Result<Vec<u8>> {
        hex::decode(&self.salt).map_err(|e| VaultError::Crypto(format!("bad salt: {e}")))
    }
}

/// A 32-byte derived key that zeroizes its memory on drop.
pub struct DerivedKey(pub [u8; 32]);

impl Drop for DerivedKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for DerivedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DerivedKey(..)")
    }
}

pub fn derive_key(password: &str, meta: &VaultMetaFile) -> Result<DerivedKey> {
    let salt = meta.salt_bytes()?;
    let params = Params::new(meta.m_cost, meta.t_cost, meta.p_cost, Some(32))
        .map_err(|e| VaultError::Crypto(format!("bad argon2 params: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), &salt, &mut out)
        .map_err(|e| VaultError::Crypto(format!("key derivation failed: {e}")))?;
    Ok(DerivedKey(out))
}

/// Encrypts `plaintext` with AES-256-GCM under `key`, returning
/// `nonce (12 bytes) || ciphertext` as a single buffer suitable for writing to disk.
pub fn encrypt(key: &DerivedKey, plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| VaultError::Crypto(format!("bad key: {e}")))?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| VaultError::Crypto("encryption failed".into()))?;
    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts a buffer produced by [`encrypt`]. Returns [`VaultError::WrongPassword`]
/// on authentication failure, which is the expected signal for "bad master password".
pub fn decrypt(key: &DerivedKey, blob: &[u8]) -> Result<Vec<u8>> {
    if blob.len() < 12 {
        return Err(VaultError::WrongPassword);
    }
    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| VaultError::Crypto(format!("bad key: {e}")))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| VaultError::WrongPassword)
}
