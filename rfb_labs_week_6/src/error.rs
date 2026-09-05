use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("wallet error: {0}")]
    Wallet(String),

    #[error("node RPC error: {0}")]
    Rpc(#[from] bitcoincore_rpc::Error),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] bdk_wallet::rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid address: {0}")]
    Address(String),

    #[error("{0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;
