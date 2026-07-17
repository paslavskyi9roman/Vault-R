use vault_core::error::{Result, VaultError};
use vault_core::Vault;

const SERVICE: &str = "vault-r";
const ACCOUNT: &str = "master-key";

/// Unlocks the vault: tries the OS keychain first (silent), falling back to
/// an interactive master-password prompt.
pub fn unlock() -> Result<Vault> {
    if !Vault::exists()? {
        return Err(VaultError::NotFound(std::path::PathBuf::from(
            "(no vault yet — run `vault init`)",
        )));
    }

    if let Ok(entry) = keyring::Entry::new(SERVICE, ACCOUNT) {
        if let Ok(key_hex) = entry.get_password() {
            if let Ok(vault) = Vault::open_with_key(&key_hex) {
                return Ok(vault);
            }
        }
    }

    let password =
        rpassword::prompt_password("Master password: ").map_err(|e| VaultError::Crypto(e.to_string()))?;
    Vault::open(&password)
}

/// Creates a brand-new vault, prompting twice for the master password.
pub fn init(remember: bool) -> Result<Vault> {
    if Vault::exists()? {
        return Err(VaultError::AlreadyExists(std::path::PathBuf::from(
            "a vault is already initialized",
        )));
    }

    let password = rpassword::prompt_password("New master password: ")
        .map_err(|e| VaultError::Crypto(e.to_string()))?;
    let confirm = rpassword::prompt_password("Confirm master password: ")
        .map_err(|e| VaultError::Crypto(e.to_string()))?;
    if password != confirm {
        return Err(VaultError::InvalidInput("passwords do not match".into()));
    }

    let vault = Vault::create(&password)?;
    if remember {
        if let Ok(entry) = keyring::Entry::new(SERVICE, ACCOUNT) {
            let _ = entry.set_password(&vault.key_hex());
        }
    }
    Ok(vault)
}
