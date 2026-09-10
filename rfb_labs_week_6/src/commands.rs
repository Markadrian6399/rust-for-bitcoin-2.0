// ============================================================================
// This file has one helper function for every trick our piggy bank can do
// (the tricks you saw listed in main.rs). Each function here is the
// "recipe" for actually doing that trick, step by step.
// ============================================================================

use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use bdk_wallet::bitcoin::{Address, Amount, FeeRate, OutPoint};
use bdk_wallet::{KeychainKind, SignOptions};

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::keys::{self, DescriptorPair};
use crate::node;
use crate::wallet;

/// Figure out which descriptor pair to use for every command *other than* `init`.
///
/// Deliberately does not fall back to generating a new mnemonic: doing so on an
/// ordinary command (e.g. `balance`) would silently open a brand new, unrelated
/// wallet instead of failing loudly, which is far more confusing than an error telling
/// the user to run `init` first.
// Before we can do almost ANY trick, we first need our address recipes.
// This little helper finds them (either from our saved secret words, or
// from a manual override) - or politely says "run `init` first!" if we
// don't have a piggy bank yet.
fn resolve_descriptors(config: &Config) -> AppResult<DescriptorPair> {
    if let (Some(external), Some(internal)) =
        (&config.external_descriptor, &config.internal_descriptor)
    {
        return Ok(DescriptorPair {
            external: external.clone(),
            internal: internal.clone(),
        });
    }

    let phrase = config.mnemonic.as_ref().ok_or_else(|| {
        AppError::Config(
            "no wallet found: run `capstone_wallet init` first, or set DESCRIPTOR / \
             CHANGE_DESCRIPTOR in .env to import an existing wallet"
                .into(),
        )
    })?;
    let mnemonic = keys::parse_mnemonic(phrase)?;
    keys::build_descriptors(
        &mnemonic,
        config.mnemonic_passphrase.clone(),
        config.network,
        config.descriptor_kind,
    )
}

// The "get a brand new piggy bank ready" trick. Picks our secret words
// (making new ones up if we don't have any yet), turns them into address
// recipes, and creates our notebook file.
pub fn init(config: &Config) -> AppResult<()> {
    let (descriptors, generated_phrase) = match (&config.mnemonic, &config.external_descriptor) {
        (_, Some(_)) => (resolve_descriptors(config)?, None),
        (Some(phrase), None) => {
            let mnemonic = keys::parse_mnemonic(phrase)?;
            let pair = keys::build_descriptors(
                &mnemonic,
                config.mnemonic_passphrase.clone(),
                config.network,
                config.descriptor_kind,
            )?;
            (pair, None)
        }
        (None, None) => {
            let mnemonic = keys::generate_mnemonic()?;
            let pair = keys::build_descriptors(
                &mnemonic,
                config.mnemonic_passphrase.clone(),
                config.network,
                config.descriptor_kind,
            )?;
            (pair, Some(mnemonic.to_string()))
        }
    };

    let (mut w, mut db) = wallet::open_or_create(config, &descriptors)?;
    let receive = w.reveal_next_address(KeychainKind::External);
    let change = w.reveal_next_address(KeychainKind::Internal);
    w.persist(&mut db)?;

    println!(
        "Network:         {}",
        wallet::network_kind_label(config.network)
    );
    println!("Descriptor kind: {:?}", config.descriptor_kind);
    println!("Wallet database: {}", config.db_path.display());
    println!("First receive address (index 0): {}", receive.address);
    println!("First change address  (index 0): {}", change.address);

    if let Some(phrase) = generated_phrase {
        persist_generated_mnemonic(&phrase)?;
        println!();
        println!("Generated a new BIP-39 seed phrase for this wallet:");
        println!();
        println!("    {phrase}");
        println!();
        println!(
            "This has been saved to .env as MNEMONIC so future commands reuse the same \
             wallet. .env is git-ignored: never commit it or share this phrase."
        );
    }

    Ok(())
}

/// Append `MNEMONIC=<phrase>` to a local `.env` file, creating it if needed. Never
/// overwrites an existing MNEMONIC entry.
// Write our brand new secret words down somewhere safe (the private .env
// file) so next time we run the program, we open the SAME piggy bank
// instead of accidentally starting a new empty one.
fn persist_generated_mnemonic(phrase: &str) -> AppResult<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let existing = std::fs::read_to_string(".env").unwrap_or_default();
    if existing
        .lines()
        .any(|l| l.trim_start().starts_with("MNEMONIC="))
    {
        return Ok(());
    }

    let mut file = OpenOptions::new().create(true).append(true).open(".env")?;
    writeln!(file, "MNEMONIC=\"{phrase}\"")?;
    Ok(())
}

// The "make a new mailbox slot" trick. `keychain` picks whether it's a
// public receiving address (share with friends) or a private change
// address (only the wallet uses it).
pub fn new_address(config: &Config, keychain: KeychainKind) -> AppResult<()> {
    let descriptors = resolve_descriptors(config)?;
    let (mut w, mut db) = wallet::open_or_create(config, &descriptors)?;
    let info = w.reveal_next_address(keychain);
    w.persist(&mut db)?;
    println!("{}", info.address);
    Ok(())
}

// The "how many coins do I have?" trick. Optionally checks in with the
// node first (`do_sync`) so the count is fresh, then adds up every coin
// we know about and prints the totals.
pub fn balance(config: &Config, do_sync: bool) -> AppResult<()> {
    let descriptors = resolve_descriptors(config)?;
    let (mut w, mut db) = wallet::open_or_create(config, &descriptors)?;

    if do_sync {
        let client = node::connect(config)?;
        let blocks = wallet::sync(&mut w, &mut db, &client, 0)?;
        eprintln!("synced {blocks} new block(s)");
    }

    let balance = w.balance();
    println!("confirmed:          {}", balance.confirmed);
    println!("trusted pending:    {}", balance.trusted_pending);
    println!("untrusted pending:  {}", balance.untrusted_pending);
    println!("immature:           {}", balance.immature);
    println!("total:              {}", balance.total());
    Ok(())
}

// The "show me every single coin" trick - like dumping the piggy bank out
// on the table and listing each coin one by one.
pub fn utxos(config: &Config) -> AppResult<()> {
    let descriptors = resolve_descriptors(config)?;
    let (w, _db) = wallet::open_or_create(config, &descriptors)?;

    for output in w.list_unspent() {
        println!(
            "{}  {:>14}  {:?}  spent={}",
            output.outpoint, output.txout.value, output.keychain, output.is_spent
        );
    }
    Ok(())
}

// The "catch me up" trick: ask the node what's new, remember it, and
// report how many new pages (blocks) we learned about.
pub fn sync(config: &Config, start_height: u32) -> AppResult<()> {
    let descriptors = resolve_descriptors(config)?;
    let (mut w, mut db) = wallet::open_or_create(config, &descriptors)?;
    let client = node::connect(config)?;
    let blocks = wallet::sync(&mut w, &mut db, &client, start_height)?;
    println!("applied {blocks} new block(s)");
    println!("balance: {}", w.balance().total());
    Ok(())
}

// The "pretend money cheat button" trick - only allowed on our practice
// playground (regtest), since real money can't just be conjured up!
pub fn mine(config: &Config, blocks: u64, address: Option<String>) -> AppResult<()> {
    if config.network != bdk_wallet::bitcoin::Network::Regtest {
        return Err(AppError::Config(
            "`mine` only makes sense on regtest (there is no faucet, so we mine our own coins)"
                .into(),
        ));
    }

    let descriptors = resolve_descriptors(config)?;
    let (mut w, mut db) = wallet::open_or_create(config, &descriptors)?;
    let client = node::connect(config)?;

    let target = match address {
        Some(a) => Address::from_str(&a)
            .map_err(|e| AppError::Address(e.to_string()))?
            .require_network(config.network)
            .map_err(|e| AppError::Address(e.to_string()))?,
        None => {
            let info = w.reveal_next_address(KeychainKind::External);
            w.persist(&mut db)?;
            info.address
        }
    };

    let hashes = node::mine_to_address(&client, blocks, &target)?;
    println!("mined {} block(s) to {target}", hashes.len());
    Ok(())
}

// The big one! The "send coins to a friend" trick. This is like: pick some
// coins from the piggy bank, write a note saying who gets what, stamp the
// note with our secret password so it's official, and mail it to the big
// shared notebook so the whole world agrees it happened.
pub fn send(
    config: &Config,
    to: String,
    amount_sats: u64,
    fee_rate_sat_vb: u64,
    manual_utxos: Vec<String>,
) -> AppResult<()> {
    let descriptors = resolve_descriptors(config)?;
    let (mut w, mut db) = wallet::open_or_create(config, &descriptors)?;
    let client = node::connect(config)?;

    // Always sync before spending so coin selection sees the node's current UTXO set.
    // Double-check our coin count is fresh before we go spending any of it -
    // like counting your piggy bank right before buying candy.
    wallet::sync(&mut w, &mut db, &client, 0)?;

    // Make sure the mailbox address is really a valid one, and belongs to
    // the same practice playground (network) we're using.
    let address = Address::from_str(&to)
        .map_err(|e| AppError::Address(e.to_string()))?
        .require_network(config.network)
        .map_err(|e| AppError::Address(e.to_string()))?;
    let amount = Amount::from_sat(amount_sats);
    let fee_rate = FeeRate::from_sat_per_vb(fee_rate_sat_vb)
        .ok_or_else(|| AppError::Config("fee rate overflowed".into()))?;

    // Start writing the note: "send `amount` coins to `address`", plus a
    // small tip (`fee_rate`) for the notebook keepers.
    let mut builder = w.build_tx();
    builder.add_recipient(address.script_pubkey(), amount);
    builder.fee_rate(fee_rate);

    // If the user pointed at specific coins to use, use exactly those and
    // nothing else. Otherwise, the wallet is smart enough to pick good
    // coins for us automatically.
    if !manual_utxos.is_empty() {
        let outpoints = manual_utxos
            .iter()
            .map(|s| {
                OutPoint::from_str(s)
                    .map_err(|e| AppError::Config(format!("invalid --utxo '{s}': {e}")))
            })
            .collect::<AppResult<Vec<_>>>()?;
        builder
            .add_utxos(&outpoints)
            .map_err(|e| AppError::Wallet(format!("failed to select utxos: {e}")))?;
        builder.manually_selected_only();
    }

    // Finish writing the note (figures out exactly which coins to use and
    // where any leftover "change" goes back to).
    let mut psbt = builder
        .finish()
        .map_err(|e| AppError::Wallet(format!("failed to build transaction: {e}")))?;
    w.persist(&mut db)?;

    // Stamp the note with our secret password, proving it's really from us.
    let finalized = w
        .sign(&mut psbt, SignOptions::default())
        .map_err(|e| AppError::Wallet(format!("failed to sign transaction: {e}")))?;
    if !finalized {
        return Err(AppError::Wallet(
            "transaction could not be fully signed/finalized".into(),
        ));
    }

    // Turn the stamped note into its final, mailable shape.
    let tx = psbt
        .extract_tx()
        .map_err(|e| AppError::Wallet(format!("failed to extract final transaction: {e}")))?;

    // Mail it! Once this succeeds, the whole network can see our note.
    let txid = node::broadcast(&client, &tx)?;

    // Track the just-broadcast tx locally so balance/utxos reflect it immediately,
    // without waiting for the next `sync`.
    // Jot it down in our own notebook right away too, so `balance`/`utxos`
    // already show the change even before we officially `sync` again.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Other(e.to_string()))?
        .as_secs();
    w.apply_unconfirmed_txs([(tx, now)]);
    w.persist(&mut db)?;

    println!("broadcast txid: {txid}");
    Ok(())
}
