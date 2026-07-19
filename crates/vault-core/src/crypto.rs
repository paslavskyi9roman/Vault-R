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

// ---------------------------------------------------------------------
// Secret generator
// ---------------------------------------------------------------------

/// The alphabet (or word list) a generated secret is drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    Hex,
    Base64Url,
    Alphanumeric,
    Passphrase,
}

const HEX_ALPHABET: &[u8] = b"0123456789abcdef";
const BASE64URL_ALPHABET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const ALPHANUMERIC_ALPHABET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Below this many characters, a "generated secret" is not meaningfully
/// harder to guess than one typed by hand.
const MIN_SECRET_CHARS: usize = 8;
/// Below this many words, a passphrase does not carry enough entropy to be
/// worth generating over just typing something.
const MIN_PASSPHRASE_WORDS: usize = 3;

/// Draws a uniformly random index in `0..n` via rejection sampling over a
/// 32-bit draw, so alphabets or word lists whose length does not evenly
/// divide a power of two are not subtly biased toward their first entries.
fn random_index(n: usize) -> usize {
    let n64 = n as u64;
    let bound = (u32::MAX as u64 + 1) / n64 * n64;
    loop {
        let mut buf = [0u8; 4];
        OsRng.fill_bytes(&mut buf);
        let v = u32::from_le_bytes(buf) as u64;
        if v < bound {
            return (v % n64) as usize;
        }
    }
}

fn random_from_alphabet(alphabet: &[u8], length: usize) -> String {
    (0..length)
        .map(|_| alphabet[random_index(alphabet.len())] as char)
        .collect()
}

fn random_passphrase(word_count: usize) -> String {
    (0..word_count)
        .map(|_| PASSPHRASE_WORDS[random_index(PASSPHRASE_WORDS.len())])
        .collect::<Vec<_>>()
        .join("-")
}

/// Generates a random secret of the given `kind`. For the character-based
/// kinds, `length` is the character count; for [`SecretKind::Passphrase`] it
/// is the word count. Rejects a `length` below a sane floor rather than
/// silently producing something too weak to be worth generating.
pub fn generate_secret(kind: SecretKind, length: usize) -> Result<String> {
    match kind {
        SecretKind::Passphrase => {
            if length < MIN_PASSPHRASE_WORDS {
                return Err(VaultError::InvalidInput(format!(
                    "a generated passphrase needs at least {MIN_PASSPHRASE_WORDS} words"
                )));
            }
            Ok(random_passphrase(length))
        }
        _ => {
            if length < MIN_SECRET_CHARS {
                return Err(VaultError::InvalidInput(format!(
                    "a generated secret needs at least {MIN_SECRET_CHARS} characters"
                )));
            }
            let alphabet = match kind {
                SecretKind::Hex => HEX_ALPHABET,
                SecretKind::Base64Url => BASE64URL_ALPHABET,
                SecretKind::Alphanumeric => ALPHANUMERIC_ALPHABET,
                SecretKind::Passphrase => unreachable!(),
            };
            Ok(random_from_alphabet(alphabet, length))
        }
    }
}

/// Short, common, unambiguous English words for passphrase generation. Not a
/// full diceware list, but large enough (~200 words, ~7.6 bits/word) that a
/// default 5-word passphrase carries meaningfully more entropy than most
/// human-chosen passwords.
const PASSPHRASE_WORDS: &[&str] = &[
    "tiger", "lion", "eagle", "otter", "panda", "koala", "zebra", "rhino", "camel", "moose",
    "beaver", "badger", "falcon", "heron", "raven", "robin", "sparrow", "swan", "crane", "stork",
    "gecko", "iguana", "turtle", "rabbit", "ferret", "weasel", "jackal", "hyena", "cougar",
    "bobcat", "lynx", "puma", "jaguar", "cheetah", "leopard", "panther", "wombat", "dingo",
    "mallard", "pelican", "osprey", "kestrel", "condor", "toucan", "parrot", "mackerel",
    "salmon", "marlin", "amber", "coral", "crimson", "scarlet", "violet", "indigo", "azure",
    "teal", "cyan", "magenta", "maroon", "olive", "ivory", "beige", "bronze", "copper", "silver",
    "slate", "river", "canyon", "meadow", "forest", "desert", "glacier", "volcano", "valley",
    "summit", "ridge", "cliff", "harbor", "lagoon", "island", "reef", "delta", "prairie",
    "tundra", "oasis", "marsh", "swamp", "cavern", "boulder", "pebble", "granite", "quartz",
    "horizon", "aurora", "thunder", "lightning", "breeze", "cyclone", "monsoon", "blizzard",
    "frost", "drizzle", "mist", "fog", "rainbow", "twilight", "dawn", "dusk", "mango", "papaya",
    "guava", "lychee", "coconut", "walnut", "almond", "hazelnut", "pistachio", "cinnamon",
    "nutmeg", "saffron", "paprika", "basil", "thyme", "rosemary", "sesame", "quinoa", "lentil",
    "oatmeal", "pretzel", "biscuit", "waffle", "pancake", "compass", "anchor", "lantern",
    "telescope", "hammer", "chisel", "wrench", "ladder", "shovel", "bucket", "kettle", "basket",
    "blanket", "pillow", "curtain", "mirror", "candle", "torch", "beacon", "anvil", "forge",
    "bellows", "spindle", "loom", "needle", "thimble", "whisper", "echo", "shadow", "glimmer",
    "spark", "ember", "flicker", "ripple", "current", "voyage", "journey", "quest", "legend",
    "fable", "riddle", "puzzle", "mosaic", "prism", "spectrum", "cascade", "cadence", "rhythm",
    "melody", "harmony", "comet", "meteor", "nebula", "galaxy", "eclipse", "zenith", "equinox",
    "solstice", "crescent", "cosmos", "tempo", "chorus", "anthem", "ballad", "sonnet", "verse",
    "canvas", "palette", "sketch", "engrave", "etching",
];

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

/// Encrypts `plaintext` with AES-256-GCM under `key`, additionally
/// authenticating (but not encrypting) `aad`. The exact same `aad` must be
/// supplied to [`decrypt_with_aad`] or authentication fails. Returns
/// `nonce (12 bytes) || ciphertext` as a single buffer suitable for writing to
/// disk. Used to bind the vault header to the database ciphertext.
pub fn encrypt_with_aad(key: &DerivedKey, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| VaultError::Crypto(format!("bad key: {e}")))?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, aes_gcm::aead::Payload { msg: plaintext, aad })
        .map_err(|_| VaultError::Crypto("encryption failed".into()))?;
    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts a buffer produced by [`encrypt_with_aad`], authenticating `aad`.
/// Returns [`VaultError::WrongPassword`] on authentication failure — a bad key
/// *or* tampered `aad` — which is the expected signal for "bad master password".
pub fn decrypt_with_aad(key: &DerivedKey, blob: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    if blob.len() < 12 {
        return Err(VaultError::WrongPassword);
    }
    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| VaultError::Crypto(format!("bad key: {e}")))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, aes_gcm::aead::Payload { msg: ciphertext, aad })
        .map_err(|_| VaultError::WrongPassword)
}

/// Encrypts with no associated data. Used for key-slot wrapping and legacy v2
/// database payloads (v3 payloads bind the header via [`encrypt_with_aad`]).
pub fn encrypt(key: &DerivedKey, plaintext: &[u8]) -> Result<Vec<u8>> {
    encrypt_with_aad(key, plaintext, &[])
}

/// Decrypts a buffer produced by [`encrypt`] (empty associated data).
pub fn decrypt(key: &DerivedKey, blob: &[u8]) -> Result<Vec<u8>> {
    decrypt_with_aad(key, blob, &[])
}

#[cfg(test)]
mod generator_tests {
    use super::*;

    #[test]
    fn hex_output_matches_length_and_alphabet() {
        let s = generate_secret(SecretKind::Hex, 32).unwrap();
        assert_eq!(s.len(), 32);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn base64url_output_matches_length_and_alphabet() {
        let s = generate_secret(SecretKind::Base64Url, 40).unwrap();
        assert_eq!(s.len(), 40);
        assert!(s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn alphanumeric_output_matches_length_and_alphabet() {
        let s = generate_secret(SecretKind::Alphanumeric, 24).unwrap();
        assert_eq!(s.len(), 24);
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn passphrase_output_has_the_requested_word_count() {
        let s = generate_secret(SecretKind::Passphrase, 5).unwrap();
        assert_eq!(s.split('-').count(), 5);
        for word in s.split('-') {
            assert!(PASSPHRASE_WORDS.contains(&word));
        }
    }

    #[test]
    fn two_calls_differ() {
        let a = generate_secret(SecretKind::Hex, 32).unwrap();
        let b = generate_secret(SecretKind::Hex, 32).unwrap();
        assert_ne!(a, b);
        let pa = generate_secret(SecretKind::Passphrase, 6).unwrap();
        let pb = generate_secret(SecretKind::Passphrase, 6).unwrap();
        assert_ne!(pa, pb);
    }

    #[test]
    fn short_lengths_are_rejected_rather_than_silently_weak() {
        assert!(generate_secret(SecretKind::Hex, 4).is_err());
        assert!(generate_secret(SecretKind::Base64Url, 1).is_err());
        assert!(generate_secret(SecretKind::Alphanumeric, 0).is_err());
        assert!(generate_secret(SecretKind::Passphrase, 1).is_err());
    }
}
