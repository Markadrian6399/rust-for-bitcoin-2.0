use std::fs;

use bdk_bitcoind_rpc::Emitter;
use bdk_wallet::bitcoin::Network;
use bdk_wallet::rusqlite::Connection;
use bdk_wallet::{KeychainKind, PersistedWallet, Wallet};
use bitcoincore_rpc::Client;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::keys::DescriptorPair;

/// Open the wallet's sqlite database, creating it (and any parent directories) on first
/// run, and either load the previously-persisted wallet or create a brand new one from
/// the given descriptors. This is what lets the wallet survive a restart: keychain
/// state, transactions and UTXOs all live in `db_path`, not just in memory.
pub fn open_or_create(
    config: &Config,
    descriptors: &DescriptorPair,
) -> AppResult<(PersistedWallet<Connection>, Connection)> {
    if let Some(parent) = config.db_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let mut db = Connection::open(&config.db_path)?;

    let loaded = Wallet::load()
        .descriptor(KeychainKind::External, Some(descriptors.external.clone()))
        .descriptor(KeychainKind::Internal, Some(descriptors.internal.clone()))
        .extract_keys()
        .check_network(config.network)
        .load_wallet(&mut db)
        .map_err(|e| AppError::Wallet(format!("failed to load wallet database: {e}")))?;

    let wallet = match loaded {
        Some(wallet) => wallet,
        None => Wallet::create(descriptors.external.clone(), descriptors.internal.clone())
            .network(config.network)
            .create_wallet(&mut db)
            .map_err(|e| AppError::Wallet(format!("failed to create wallet database: {e}")))?,
    };

    Ok((wallet, db))
}

/// Sync the wallet against a Bitcoin Core node via `bitcoincore-rpc`.
///
/// This is the wallet's only connection to the outside world: it walks the node's
/// active chain block-by-block from the wallet's last-known tip (handling reorgs via
/// `connected_to`), applies each block's relevant transactions, then folds in the
/// current mempool. Every applied block/mempool update is persisted immediately so a
/// crash mid-sync loses at most one step of progress.
pub fn sync(
    wallet: &mut PersistedWallet<Connection>,
    db: &mut Connection,
    client: &Client,
    start_height: u32,
) -> AppResult<u32> {
    let wallet_tip = wallet.latest_checkpoint();
    let unconfirmed_txids = wallet
        .transactions()
        .filter(|tx| tx.chain_position.is_unconfirmed());

    let mut emitter = Emitter::new(client, wallet_tip, start_height, unconfirmed_txids);

    let mut blocks_applied = 0u32;
    while let Some(block_emission) = emitter
        .next_block()
        .map_err(|e| AppError::Wallet(format!("failed to fetch block from node: {e:?}")))?
    {
        let height = block_emission.block_height();
        let connected_to = block_emission.connected_to();
        wallet
            .apply_block_connected_to(&block_emission.block, height, connected_to)
            .map_err(|e| AppError::Wallet(format!("failed to apply block {height}: {e}")))?;
        wallet.persist(db)?;
        blocks_applied += 1;
    }

    let mempool_event = emitter
        .mempool()
        .map_err(|e| AppError::Wallet(format!("failed to fetch mempool from node: {e:?}")))?;
    wallet.apply_evicted_txs(mempool_event.evicted);
    wallet.apply_unconfirmed_txs(mempool_event.update);
    wallet.persist(db)?;

    Ok(blocks_applied)
}

pub fn network_kind_label(network: Network) -> &'static str {
    match network {
        Network::Bitcoin => "mainnet",
        Network::Testnet | Network::Testnet4 => "testnet",
        Network::Signet => "signet",
        Network::Regtest => "regtest",
        #[allow(unreachable_patterns)]
        _ => "unknown",
    }
}
