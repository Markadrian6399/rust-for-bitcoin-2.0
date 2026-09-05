mod commands;
mod config;
mod error;
mod keys;
mod node;
mod wallet;

use bdk_wallet::KeychainKind;
use clap::{Parser, Subcommand};

use config::Config;
use error::AppResult;

/// A descriptor-based Bitcoin wallet for regtest/testnet, built on BDK, rust-bitcoin,
/// and bitcoincore-rpc.
#[derive(Parser)]
#[command(name = "capstone_wallet", author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate or import keys, derive descriptors, and create the wallet database.
    Init,
    /// Reveal a new receiving (external keychain) address.
    Address,
    /// Reveal a new change (internal keychain) address.
    ChangeAddress,
    /// Sync wallet state against the configured Bitcoin Core node.
    Sync {
        /// Block height to start scanning from (0 = from genesis / wallet birthday).
        #[arg(long, default_value_t = 0)]
        start_height: u32,
    },
    /// Print the wallet balance.
    Balance {
        /// Sync with the node before reporting the balance.
        #[arg(long)]
        sync: bool,
    },
    /// List tracked UTXOs.
    Utxos,
    /// Build, sign, and broadcast a transaction.
    Send {
        /// Destination address.
        #[arg(long)]
        to: String,
        /// Amount to send, in satoshis.
        #[arg(long)]
        amount: u64,
        /// Fee rate in sat/vB.
        #[arg(long, default_value_t = 1)]
        fee_rate: u64,
        /// Spend only these UTXOs (format: txid:vout). Repeatable. When set, disables
        /// automatic coin selection entirely.
        #[arg(long = "utxo")]
        utxos: Vec<String>,
    },
    /// Regtest-only: mine blocks (there is no faucet, so this is how you fund the wallet).
    Mine {
        #[arg(long, default_value_t = 1)]
        blocks: u64,
        /// Address to pay the coinbase reward to; defaults to a fresh wallet address.
        #[arg(long)]
        address: Option<String>,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        Command::Init => commands::init(&config),
        Command::Address => commands::new_address(&config, KeychainKind::External),
        Command::ChangeAddress => commands::new_address(&config, KeychainKind::Internal),
        Command::Sync { start_height } => commands::sync(&config, start_height),
        Command::Balance { sync } => commands::balance(&config, sync),
        Command::Utxos => commands::utxos(&config),
        Command::Send {
            to,
            amount,
            fee_rate,
            utxos,
        } => commands::send(&config, to, amount, fee_rate, utxos),
        Command::Mine { blocks, address } => commands::mine(&config, blocks, address),
    }
}
