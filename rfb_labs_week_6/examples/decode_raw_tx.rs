//! Demonstrates reaching for raw `rust-bitcoin` instead of BDK.
//!
//! BDK's `Wallet` only knows how to interpret transactions that touch its own
//! descriptors - there is no BDK API for "decode and print an arbitrary raw transaction
//! hex", because that isn't a wallet operation, it's a codec operation. For that we drop
//! straight to `rust-bitcoin`'s consensus decoder, the same primitive this repo's Week 3
//! `decodetrx` lab was built on.
//!
//! Run against a local regtest node:
//!   cargo run --example decode_raw_tx -- <txid>
//!
//! ------------------------------------------------------------------------
//! In plain words: a Bitcoin "note" (transaction) is written in a special
//! secret code (bytes) that computers understand but people don't. This
//! little program is a translator: you give it a note's ID number (txid),
//! it asks the big shared notebook keeper for the note's secret-code
//! version, and then it translates that code into a friendly list like
//! "this much money came FROM here, and went TO there."

use std::env;

use bitcoin::consensus::encode::deserialize_hex;
use bitcoin::Transaction;
use bitcoincore_rpc::{Auth, Client, RpcApi};

fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let txid = env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: decode_raw_tx <txid>"))?;

    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "127.0.0.1:18443".into());
    let auth = match (env::var("RPC_USER"), env::var("RPC_PASS")) {
        (Ok(user), Ok(pass)) => Auth::UserPass(user, pass),
        _ => Auth::None,
    };
    let client = Client::new(&rpc_url, auth)?;

    // getrawtransaction returns hex; bitcoincore-rpc doesn't parse it into a
    // rust-bitcoin `Transaction` for us here, so we decode it ourselves.
    // Ask the notebook keeper: "what does note number `txid` look like, in
    // secret code?" It answers with a long string of letters and numbers
    // (hex) - that IS the note, just not readable by humans yet.
    let raw_hex: String = client.call("getrawtransaction", &[txid.into()])?;
    // Translate that secret code into a friendly Rust `Transaction` we can
    // actually read and print pieces of.
    let tx: Transaction = deserialize_hex(&raw_hex)?;

    println!("txid:     {}", tx.compute_txid());
    println!("version:  {}", tx.version);
    println!("locktime: {}", tx.lock_time);
    println!("weight:   {}", tx.weight());
    println!();

    // "Inputs" are the coins this note is SPENDING (coming in).
    for (i, input) in tx.input.iter().enumerate() {
        println!("input[{i}]");
        println!("  previous_output: {}", input.previous_output);
        println!("  sequence:        {:?}", input.sequence);
        println!("  script_sig asm:  {}", input.script_sig.to_asm_string());
        if !input.witness.is_empty() {
            println!("  witness items:   {}", input.witness.len());
        }
    }

    // "Outputs" are the new coins this note CREATES (going out to
    // whoever's mailbox address is on them).
    for (i, output) in tx.output.iter().enumerate() {
        println!("output[{i}]");
        println!("  value:         {}", output.value);
        println!("  script asm:    {}", output.script_pubkey.to_asm_string());
        println!(
            "  script type:   {}",
            if output.script_pubkey.is_p2wpkh() {
                "p2wpkh"
            } else if output.script_pubkey.is_p2tr() {
                "p2tr"
            } else if output.script_pubkey.is_p2wsh() {
                "p2wsh"
            } else if output.script_pubkey.is_p2pkh() {
                "p2pkh"
            } else if output.script_pubkey.is_p2sh() {
                "p2sh"
            } else {
                "other"
            }
        );
    }

    Ok(())
}
