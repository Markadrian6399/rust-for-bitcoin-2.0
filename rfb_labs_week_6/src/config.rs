// ============================================================================
// This file is like a recipe card. Before our piggy bank can do anything,
// it needs to know a few things: which pretend-money playground to use
// (regtest), where to save its notebook, and how to reach the big shared
// notebook (the node). Instead of writing those answers directly into the
// code (which anyone reading the code could then see - not safe for secrets
// like passwords!), we read them from a `.env` file that stays private on
// your own computer.
// ============================================================================

use std::env;
use std::path::PathBuf;

use bdk_wallet::bitcoin::Network;

use crate::error::{AppError, AppResult};

// Two different "shapes" of address our wallet can make. Think of these
// like two different rubber stamps that both make a valid mailbox address,
// just with slightly different-looking envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorKind {
    /// BIP84-style native segwit v0 (wpkh) keychain.
    Wpkh,
    /// BIP86-style taproot (tr) keychain.
    Taproot,
}

impl DescriptorKind {
    fn from_env(value: &str) -> AppResult<Self> {
        match value.to_ascii_lowercase().as_str() {
            "wpkh" | "segwit" | "bip84" => Ok(Self::Wpkh),
            "tr" | "taproot" | "bip86" => Ok(Self::Taproot),
            other => Err(AppError::Config(format!(
                "unknown DESCRIPTOR_KIND '{other}', expected 'wpkh' or 'taproot'"
            ))),
        }
    }
}

/// Wallet + node configuration, loaded from the environment (and `.env` if present).
///
/// Nothing here is hardcoded: secrets such as the seed phrase and RPC credentials
/// only ever live in the environment / `.env` file, which is git-ignored.
// All the answers to our recipe card questions, gathered up in one box so
// every other part of the program can just ask "hey Config, what's the
// database path?" instead of re-reading the .env file itself.
pub struct Config {
    pub network: Network,
    pub db_path: PathBuf,
    pub descriptor_kind: DescriptorKind,

    pub rpc_url: String,
    pub rpc_user: Option<String>,
    pub rpc_pass: Option<String>,
    pub rpc_cookie: Option<PathBuf>,

    /// BIP-39 mnemonic. If absent, `init` generates a fresh one.
    /// 12 secret words that ARE the piggy bank's password. Anyone who knows
    /// these words could unlock the coins, so this must never be written
    /// into code or shared - only ever kept in the private `.env` file.
    pub mnemonic: Option<String>,
    pub mnemonic_passphrase: Option<String>,

    /// Optional explicit descriptor override, bypassing mnemonic derivation entirely.
    pub external_descriptor: Option<String>,
    pub internal_descriptor: Option<String>,
}

impl Config {
    // Go read the recipe card (the .env file and/or your computer's
    // environment variables) and fill in every answer above.
    pub fn load() -> AppResult<Self> {
        // Best-effort: fine if there is no .env file (e.g. CI, or vars set directly).
        let _ = dotenvy::dotenv();

        // Which pretend/practice playground are we in? Default to "regtest",
        // our own private sandbox where we can make as much fake money as
        // we like.
        let network = match env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".into()).as_str() {
            "regtest" => Network::Regtest,
            "testnet" => Network::Testnet,
            "signet" => Network::Signet,
            other => {
                return Err(AppError::Config(format!(
                    "unsupported BITCOIN_NETWORK '{other}'; this wallet only supports regtest/testnet/signet"
                )))
            }
        };

        // Where do we keep our notebook (the little database file) that
        // remembers everything about our coins, even after we close the
        // program?
        let db_path = env::var("WALLET_DB_PATH")
            .unwrap_or_else(|_| "wallet_data/wallet.sqlite".into())
            .into();

        let descriptor_kind = match env::var("DESCRIPTOR_KIND") {
            Ok(v) => DescriptorKind::from_env(&v)?,
            Err(_) => DescriptorKind::Wpkh,
        };

        // How do we phone the big shared notebook (the Bitcoin node)? Plus
        // the password to prove we're allowed to ask it things.
        let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "127.0.0.1:18443".into());
        let rpc_user = env::var("RPC_USER").ok();
        let rpc_pass = env::var("RPC_PASS").ok();
        let rpc_cookie = env::var("RPC_COOKIE").ok().map(PathBuf::from);

        // Our secret 12 words (if we already have some) and an optional
        // extra secret word on top, like a password with a bonus PIN.
        let mnemonic = env::var("MNEMONIC").ok();
        let mnemonic_passphrase = env::var("MNEMONIC_PASSPHRASE").ok();

        // Grown-up escape hatch: skip the secret words entirely and just
        // tell the wallet exactly which addresses to use.
        let external_descriptor = env::var("DESCRIPTOR").ok();
        let internal_descriptor = env::var("CHANGE_DESCRIPTOR").ok();

        Ok(Self {
            network,
            db_path,
            descriptor_kind,
            rpc_url,
            rpc_user,
            rpc_pass,
            rpc_cookie,
            mnemonic,
            mnemonic_passphrase,
            external_descriptor,
            internal_descriptor,
        })
    }
}
