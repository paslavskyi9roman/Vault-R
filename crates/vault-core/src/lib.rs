pub mod backup;
pub mod crypto;
pub mod db;
pub mod dotenv;
pub mod error;
pub mod gitguard;
pub mod health;
pub mod models;
pub mod paths;
mod store;

pub use db::{Vault, VaultStatus};
pub use error::{Result, VaultError};
