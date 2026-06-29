//! btctax-store: PGP-encrypted local vault for the bitcoin_tax ledger.
pub const SCHEMA_VERSION: u32 = 1;

pub mod atomic;
pub mod blob;
pub mod crypto;
pub mod fsperms;
pub mod lock;
pub mod memlock;
pub mod paths;
pub mod sqlite_io;
pub mod vault;

pub use crypto::Passphrase;
pub use vault::Vault;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("openpgp: {0}")]
    Crypto(#[from] anyhow::Error),
    #[error("another instance holds the vault lock")]
    Locked,
    #[error("wrong passphrase or corrupt key")]
    WrongPassphrase,
    #[error("vault blob is corrupt: {0}")]
    Corrupt(String),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("unsupported schema version {0}")]
    UnsupportedSchema(u32),
    #[error("vault already exists at this path")]
    AlreadyExists,
    #[error("invalid vault path (must not end in .key)")]
    InvalidVaultPath,
}
