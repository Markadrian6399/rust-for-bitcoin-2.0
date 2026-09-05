use std::env;
use std::path::PathBuf;

use bdk_wallet::bitcoin::Network;

use crate::error::{AppError, AppResult};

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
pub struct Config {
    pub network: Network,
    pub db_path: PathBuf,
    pub descriptor_kind: DescriptorKind,

    pub rpc_url: String,
    pub rpc_user: Option<String>,
    pub rpc_pass: Option<String>,
    pub rpc_cookie: Option<PathBuf>,

    /// BIP-39 mnemonic. If absent, `init` generates a fresh one.
    pub mnemonic: Option<String>,
    pub mnemonic_passphrase: Option<String>,

    /// Optional explicit descriptor override, bypassing mnemonic derivation entirely.
    pub external_descriptor: Option<String>,
    pub internal_descriptor: Option<String>,
}

impl Config {
    pub fn load() -> AppResult<Self> {
        // Best-effort: fine if there is no .env file (e.g. CI, or vars set directly).
        let _ = dotenvy::dotenv();

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

        let db_path = env::var("WALLET_DB_PATH")
            .unwrap_or_else(|_| "wallet_data/wallet.sqlite".into())
            .into();

        let descriptor_kind = match env::var("DESCRIPTOR_KIND") {
            Ok(v) => DescriptorKind::from_env(&v)?,
            Err(_) => DescriptorKind::Wpkh,
        };

        let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "127.0.0.1:18443".into());
        let rpc_user = env::var("RPC_USER").ok();
        let rpc_pass = env::var("RPC_PASS").ok();
        let rpc_cookie = env::var("RPC_COOKIE").ok().map(PathBuf::from);

        let mnemonic = env::var("MNEMONIC").ok();
        let mnemonic_passphrase = env::var("MNEMONIC_PASSPHRASE").ok();

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
