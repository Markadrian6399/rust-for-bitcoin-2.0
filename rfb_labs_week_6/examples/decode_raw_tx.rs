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
    let raw_hex: String = client.call("getrawtransaction", &[txid.into()])?;
    let tx: Transaction = deserialize_hex(&raw_hex)?;

    println!("txid:     {}", tx.compute_txid());
    println!("version:  {}", tx.version);
    println!("locktime: {}", tx.lock_time);
    println!("weight:   {}", tx.weight());
    println!();

    for (i, input) in tx.input.iter().enumerate() {
        println!("input[{i}]");
        println!("  previous_output: {}", input.previous_output);
        println!("  sequence:        {:?}", input.sequence);
        println!("  script_sig asm:  {}", input.script_sig.to_asm_string());
        if !input.witness.is_empty() {
            println!("  witness items:   {}", input.witness.len());
        }
    }

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
