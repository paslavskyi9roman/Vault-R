use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("no vault exists yet at {0}")]
    NotFound(PathBuf),
    #[error("a vault already exists at {0}")]
    AlreadyExists(PathBuf),
    #[error("incorrect master password")]
    WrongPassword,
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("not found: {0}")]
    Missing(String),
    #[error("already exists: {0}")]
    Duplicate(String),
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, VaultError>;
