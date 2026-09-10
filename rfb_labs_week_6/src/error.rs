// ============================================================================
// This file is our "uh oh" box. 📦
//
// Instead of the program just crashing with a scary wall of text whenever
// something goes wrong (like the node being asleep, or you typing a silly
// address), every "something went wrong" gets turned into one of these
// tidy, labeled boxes, with a short, human sentence describing what
// happened. Then `main.rs` just prints that sentence and stops nicely.
// ============================================================================

use thiserror::Error;

// Every different kind of "oops" our wallet might run into, each with its
// own friendly explanation.
#[derive(Debug, Error)]
pub enum AppError {
    /// Something about our settings (.env file) doesn't make sense.
    #[error("configuration error: {0}")]
    Config(String),

    /// The piggy-bank logic itself hit a snag (e.g. couldn't build/sign a note).
    #[error("wallet error: {0}")]
    Wallet(String),

    /// Couldn't talk to the big shared notebook (the node) properly.
    #[error("node RPC error: {0}")]
    Rpc(#[from] bitcoincore_rpc::Error),

    /// Trouble reading or writing our own notebook file on disk.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] bdk_wallet::rusqlite::Error),

    /// A regular file/disk problem (e.g. couldn't create a folder).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Someone typed a mailbox address that isn't a real address.
    #[error("invalid address: {0}")]
    Address(String),

    /// A catch-all box for anything else that doesn't fit above.
    #[error("{0}")]
    Other(String),
}

// A shorthand: "this function either gives back a good answer, or one of
// our tidy AppError boxes explaining what went wrong."
pub type AppResult<T> = Result<T, AppError>;
