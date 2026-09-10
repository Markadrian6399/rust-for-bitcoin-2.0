// ============================================================================
// Welcome! This is the front door of our pretend piggy bank program. 🐷💰
//
// Think of this whole project like a magic piggy bank that:
//   1. Knows a secret password (made of 12 words) that only YOU know.
//   2. Can make brand new mailbox addresses so friends can send it coins.
//   3. Keeps count of every coin inside it.
//   4. Remembers everything even after you turn the computer off.
//   5. Can write little notes that say "send some coins to my friend!",
//      stamp them with the secret password so everyone believes they're
//      really from us, and mail them out.
//   6. Talks to a big shared notebook (the Bitcoin node) to check what's
//      real and to mail out our notes.
//
// This file just listens for what YOU typed in the terminal (like "balance"
// or "send") and then asks the right helper (in the other files) to do it.
// ============================================================================

mod commands; // the "do the thing" helpers, one per command
mod config; // reads settings (like a recipe card) from the .env file
mod error; // one tidy box to put every kind of "uh oh, something broke" in
mod keys; // makes the secret password and turns it into addresses
mod node; // how we talk to the big shared notebook (the Bitcoin node)
mod wallet; // opens/saves our piggy bank and asks the notebook for updates

use bdk_wallet::KeychainKind;
use clap::{Parser, Subcommand};

use config::Config;
use error::AppResult;

/// A descriptor-based Bitcoin wallet for regtest/testnet, built on BDK, rust-bitcoin,
/// and bitcoincore-rpc.
// This tells the `clap` library: "please read whatever the user typed after
// the program name, and figure out which Command (below) they meant."
#[derive(Parser)]
#[command(name = "capstone_wallet", author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

// These are all the "tricks" our piggy bank can do. Typing e.g.
// `cargo run -- balance` picks the `Balance` trick from this list.
#[derive(Subcommand)]
enum Command {
    /// Generate or import keys, derive descriptors, and create the wallet database.
    /// Like the very first time you get a piggy bank: pick (or make up) a
    /// secret password and get it ready to hold coins.
    Init,
    /// Reveal a new receiving (external keychain) address.
    /// Makes a brand new mailbox slot so someone can send us coins.
    Address,
    /// Reveal a new change (internal keychain) address.
    /// Makes a private mailbox slot just for OUR OWN leftover coins
    /// ("change") to come back to, kind of like the change you get back
    /// after paying for candy.
    ChangeAddress,
    /// Sync wallet state against the configured Bitcoin Core node.
    /// Ask the big shared notebook "what's new since I last looked?" and
    /// copy those pages into our own notebook.
    Sync {
        /// Block height to start scanning from (0 = from genesis / wallet birthday).
        #[arg(long, default_value_t = 0)]
        start_height: u32,
    },
    /// Print the wallet balance.
    /// Count all our coins and say the total out loud.
    Balance {
        /// Sync with the node before reporting the balance.
        #[arg(long)]
        sync: bool,
    },
    /// List tracked UTXOs.
    /// Show every single coin we own, one by one (like dumping the piggy
    /// bank on the table and lining up each coin).
    Utxos,
    /// Build, sign, and broadcast a transaction.
    /// Write a note ("send this many coins to this mailbox"), stamp it with
    /// our secret password so it's official, and mail it to the notebook.
    Send {
        /// Destination address.
        #[arg(long)]
        to: String,
        /// Amount to send, in satoshis.
        #[arg(long)]
        amount: u64,
        /// Fee rate in sat/vB.
        /// A tiny tip we pay the notebook-keepers for handling our note.
        #[arg(long, default_value_t = 1)]
        fee_rate: u64,
        /// Spend only these UTXOs (format: txid:vout). Repeatable. When set, disables
        /// automatic coin selection entirely.
        /// Normally the wallet picks which coins to spend for you; this lets
        /// you point at exact coins yourself, like saying "use THIS coin,
        /// not that one."
        #[arg(long = "utxo")]
        utxos: Vec<String>,
    },
    /// Regtest-only: mine blocks (there is no faucet, so this is how you fund the wallet).
    /// A pretend-money cheat button that only works in our practice sandbox
    /// (regtest): it makes new blocks appear and gives us fake coins to
    /// play with, since there's no real store to buy test coins from.
    Mine {
        #[arg(long, default_value_t = 1)]
        blocks: u64,
        /// Address to pay the coinbase reward to; defaults to a fresh wallet address.
        #[arg(long)]
        address: Option<String>,
    },
}

// The very first thing that runs when you start the program.
fn main() {
    // Try to do the thing the user asked for. If anything went wrong,
    // print a friendly-ish error message instead of crashing with scary text.
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

// This is the "traffic director": it reads what trick you asked for and
// sends you to the right helper function to actually do it.
fn run() -> AppResult<()> {
    let cli = Cli::parse(); // figure out which Command the user typed
    let config = Config::load()?; // read our settings (.env file) first

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
