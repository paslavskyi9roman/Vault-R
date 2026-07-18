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

/// One way to unlock the vault. A slot stores the Argon2id parameters used to
/// turn a secret (master password, recovery code) into a *wrapping* key, plus
/// the vault's data key encrypted under it. Adding a slot therefore grants a
/// new way in without re-encrypting the database, and changing the master
/// password only rewrites the password slot.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeySlot {
    /// hex-encoded 16-byte salt
    pub salt: String,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
    /// hex-encoded `nonce || AES-256-GCM(wrapping_key, data_key)`
    pub wrapped_key: String,
}

impl KeySlot {
    /// Derives a fresh wrapping key from `secret` and wraps `data_key` under it.
    pub fn seal(secret: &str, data_key: &DerivedKey) -> Result<Self> {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let meta = VaultMetaFile {
            version: 2,
            salt: hex::encode(salt),
            m_cost: DEFAULT_M_COST,
            t_cost: DEFAULT_T_COST,
            p_cost: DEFAULT_P_COST,
        };
        let wrapping = derive_key(secret, &meta)?;
        let wrapped = encrypt(&wrapping, &data_key.0)?;
        Ok(KeySlot {
            salt: meta.salt,
            m_cost: meta.m_cost,
            t_cost: meta.t_cost,
            p_cost: meta.p_cost,
            wrapped_key: hex::encode(wrapped),
        })
    }

    /// Recovers the data key from this slot. Returns [`VaultError::WrongPassword`]
    /// if `secret` is not the one the slot was sealed with.
    pub fn open(&self, secret: &str) -> Result<DerivedKey> {
        let meta = VaultMetaFile {
            version: 2,
            salt: self.salt.clone(),
            m_cost: self.m_cost,
            t_cost: self.t_cost,
            p_cost: self.p_cost,
        };
        let wrapping = derive_key(secret, &meta)?;
        let wrapped =
            hex::decode(&self.wrapped_key).map_err(|e| VaultError::Crypto(e.to_string()))?;
        let mut plain = decrypt(&wrapping, &wrapped)?;
        let key = DerivedKey::from_slice(&plain)?;
        plain.zeroize();
        Ok(key)
    }
}

/// The plaintext header of a v2 vault file: which secrets can unlock it.
/// It is authenticated only in the sense that a tampered slot simply fails to
/// unwrap — the data key, and therefore the database, stays sealed.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultHeader {
    pub version: u32,
    pub password: KeySlot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<KeySlot>,
}

/// A 32-byte derived key that zeroizes its memory on drop.
pub struct DerivedKey(pub [u8; 32]);

impl DerivedKey {
    /// A brand-new random key. Used for the vault's data key, which is never
    /// derived from anything the user types.
    pub fn random() -> Self {
        let mut out = [0u8; 32];
        OsRng.fill_bytes(&mut out);
        DerivedKey(out)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(VaultError::Crypto("key has wrong length".into()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(DerivedKey(arr))
    }

    pub fn from_hex(key_hex: &str) -> Result<Self> {
        let mut bytes = hex::decode(key_hex).map_err(|e| VaultError::Crypto(e.to_string()))?;
        let key = Self::from_slice(&bytes);
        bytes.zeroize();
        key
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

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

/// Crockford base32: no I, L, O or U, so the code has no character pairs a
/// user can confuse when copying it off a printed recovery sheet.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A fresh 125-bit recovery code, formatted as five groups of five.
pub fn new_recovery_code() -> String {
    let mut raw = [0u8; 25];
    OsRng.fill_bytes(&mut raw);
    let chars: Vec<char> = raw
        .iter()
        .map(|b| CROCKFORD[(b % 32) as usize] as char)
        .collect();
    chars
        .chunks(5)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// Canonicalizes a user-typed recovery code: case is ignored, separators and
/// whitespace are dropped, and the Crockford look-alikes are folded (I/L to 1,
/// O to 0) so a transcription slip still unlocks the vault.
pub fn normalize_recovery_code(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| match c.to_ascii_uppercase() {
            'I' | 'L' => '1',
            'O' => '0',
            other => other,
        })
        .collect()
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
