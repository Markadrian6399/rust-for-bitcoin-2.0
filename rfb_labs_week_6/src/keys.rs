use std::str::FromStr;

use bdk_wallet::bitcoin::bip32::DerivationPath;
use bdk_wallet::bitcoin::secp256k1::Secp256k1;
use bdk_wallet::bitcoin::{Network, NetworkKind};
use bdk_wallet::descriptor;
use bdk_wallet::descriptor::IntoWalletDescriptor;
use bdk_wallet::keys::bip39::{Language, Mnemonic, WordCount};
use bdk_wallet::keys::{GeneratableKey, GeneratedKey};
use bdk_wallet::miniscript;
use bdk_wallet::miniscript::Tap;

use crate::config::DescriptorKind;
use crate::error::{AppError, AppResult};

/// A pair of ready-to-use descriptor strings (with embedded private keys) for the
/// external (receiving) and internal (change) keychains.
pub struct DescriptorPair {
    pub external: String,
    pub internal: String,
}

/// Generate a fresh 12-word BIP-39 mnemonic.
pub fn generate_mnemonic() -> AppResult<Mnemonic> {
    let generated: GeneratedKey<_, Tap> =
        Mnemonic::generate((WordCount::Words12, Language::English))
            .map_err(|e| AppError::Wallet(format!("mnemonic generation failed: {e:?}")))?;
    Ok((*generated).clone())
}

/// Parse an existing mnemonic phrase (e.g. loaded from `.env`).
pub fn parse_mnemonic(phrase: &str) -> AppResult<Mnemonic> {
    Mnemonic::parse_in(Language::English, phrase)
        .map_err(|e| AppError::Config(format!("invalid MNEMONIC: {e}")))
}

/// Derive external/internal descriptors (with private key material) from a mnemonic.
///
/// Uses BIP84 derivation paths (`m/84'/{coin}'/0'/{0,1}`) for `wpkh` wallets and BIP86
/// paths (`m/86'/{coin}'/0'/{0,1}`) for `tr` (taproot) wallets, matching the account
/// structure real wallets use so external and internal keychains never collide.
pub fn build_descriptors(
    mnemonic: &Mnemonic,
    passphrase: Option<String>,
    network: Network,
    kind: DescriptorKind,
) -> AppResult<DescriptorPair> {
    let secp = Secp256k1::new();
    let network_kind = NetworkKind::from(network);
    // BIP44-style coin type: 0' for mainnet, 1' for any test network.
    let coin_type = if network_kind.is_mainnet() { 0 } else { 1 };

    let source = (mnemonic.clone(), passphrase);

    match kind {
        DescriptorKind::Wpkh => {
            let external_path =
                DerivationPath::from_str(&format!("m/84h/{coin_type}h/0h/0")).expect("valid path");
            let internal_path =
                DerivationPath::from_str(&format!("m/84h/{coin_type}h/0h/1")).expect("valid path");

            let (external, ext_keymap) = descriptor!(wpkh((source.clone(), external_path)))
                .map_err(|e| AppError::Wallet(format!("descriptor build failed: {e}")))?
                .into_wallet_descriptor(&secp, network_kind)
                .map_err(|e| AppError::Wallet(format!("descriptor resolve failed: {e}")))?;
            let (internal, int_keymap) = descriptor!(wpkh((source, internal_path)))
                .map_err(|e| AppError::Wallet(format!("descriptor build failed: {e}")))?
                .into_wallet_descriptor(&secp, network_kind)
                .map_err(|e| AppError::Wallet(format!("descriptor resolve failed: {e}")))?;

            Ok(DescriptorPair {
                external: external.to_string_with_secret(&ext_keymap),
                internal: internal.to_string_with_secret(&int_keymap),
            })
        }
        DescriptorKind::Taproot => {
            let external_path =
                DerivationPath::from_str(&format!("m/86h/{coin_type}h/0h/0")).expect("valid path");
            let internal_path =
                DerivationPath::from_str(&format!("m/86h/{coin_type}h/0h/1")).expect("valid path");

            let (external, ext_keymap) = descriptor!(tr((source.clone(), external_path)))
                .map_err(|e| AppError::Wallet(format!("descriptor build failed: {e}")))?
                .into_wallet_descriptor(&secp, network_kind)
                .map_err(|e| AppError::Wallet(format!("descriptor resolve failed: {e}")))?;
            let (internal, int_keymap) = descriptor!(tr((source, internal_path)))
                .map_err(|e| AppError::Wallet(format!("descriptor build failed: {e}")))?
                .into_wallet_descriptor(&secp, network_kind)
                .map_err(|e| AppError::Wallet(format!("descriptor resolve failed: {e}")))?;

            Ok(DescriptorPair {
                external: external.to_string_with_secret(&ext_keymap),
                internal: internal.to_string_with_secret(&int_keymap),
            })
        }
    }
}
