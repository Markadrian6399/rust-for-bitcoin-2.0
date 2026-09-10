// ============================================================================
// This file is the "magic password maker." 🔑
//
// Imagine your secret password isn't just one word, but 12 simple words in
// a row, like: "apple river dog moon ...". That list of words is called a
// "mnemonic" (say it: nuh-MON-ik), and it's just an easy-for-humans way of
// writing down a REALLY big secret number.
//
// From that one list of 12 words, we can mathematically create as many
// secret keys and addresses as we want - forever, always the same ones,
// as long as we remember the same 12 words. That's why those words are so
// precious: lose them and you lose the piggy bank forever; let someone
// else see them and they can open your piggy bank too!
//
// We make TWO separate "families" of addresses from the same 12 words:
//   - the "external" family: addresses we hand out so people can pay us
//   - the "internal" family: addresses only WE use, to get our own
//     leftover change back
// Keeping them separate is like having a mail slot for letters coming IN
// versus a private drawer for money that bounces back to yourself.
// ============================================================================

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
// A "descriptor" is like a recipe that says exactly how to make an
// endless list of addresses from our secret words. This struct just holds
// the two recipes: one for handing-out addresses, one for change addresses.
pub struct DescriptorPair {
    pub external: String,
    pub internal: String,
}

/// Generate a fresh 12-word BIP-39 mnemonic.
// Roll the dice (very, very securely!) to come up with 12 brand new secret
// words nobody has ever used before. This is like getting a fresh,
// never-opened piggy bank with a password only you will ever know.
pub fn generate_mnemonic() -> AppResult<Mnemonic> {
    let generated: GeneratedKey<_, Tap> =
        Mnemonic::generate((WordCount::Words12, Language::English))
            .map_err(|e| AppError::Wallet(format!("mnemonic generation failed: {e:?}")))?;
    Ok((*generated).clone())
}

/// Parse an existing mnemonic phrase (e.g. loaded from `.env`).
// If we already have 12 secret words written down (from last time), read
// them back in so we can unlock the SAME piggy bank again.
pub fn parse_mnemonic(phrase: &str) -> AppResult<Mnemonic> {
    Mnemonic::parse_in(Language::English, phrase)
        .map_err(|e| AppError::Config(format!("invalid MNEMONIC: {e}")))
}

/// Derive external/internal descriptors (with private key material) from a mnemonic.
///
/// Uses BIP84 derivation paths (`m/84'/{coin}'/0'/{0,1}`) for `wpkh` wallets and BIP86
/// paths (`m/86'/{coin}'/0'/{0,1}`) for `tr` (taproot) wallets, matching the account
/// structure real wallets use so external and internal keychains never collide.
// Turn our 12 secret words into the two address-making recipes described
// above. Think of it like: same secret ingredients, but two different
// cookie cutters (external vs. internal) so the cookies never get mixed up.
pub fn build_descriptors(
    mnemonic: &Mnemonic,
    passphrase: Option<String>,
    network: Network,
    kind: DescriptorKind,
) -> AppResult<DescriptorPair> {
    let secp = Secp256k1::new();
    let network_kind = NetworkKind::from(network);
    // BIP44-style coin type: 0' for mainnet, 1' for any test network.
    // Like writing "REAL money" vs "PRETEND money" on the recipe card so we
    // never accidentally mix up practice coins with the real thing.
    let coin_type = if network_kind.is_mainnet() { 0 } else { 1 };

    // Bundle up the secret words + optional bonus word - this is the
    // "dough" we'll cut into address "cookies" below.
    let source = (mnemonic.clone(), passphrase);

    match kind {
        // "wpkh" = the classic, simple, everybody-supports-it address shape.
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
        // "tr" (taproot) = a newer, fancier address shape.
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
