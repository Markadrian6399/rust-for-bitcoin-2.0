# Week 6 Capstone: Bitcoin Wallet (`rfb_labs_week_6`)

See [ASSIGNMENT.md](ASSIGNMENT.md) for the full brief this implements.

A descriptor-based Bitcoin wallet for regtest/testnet, built on
[`bdk_wallet`](https://docs.rs/bdk_wallet), [`rust-bitcoin`](https://docs.rs/bitcoin),
and [`bitcoincore-rpc`](https://docs.rs/bitcoincore-rpc). It generates keys, derives
separate receiving/change keychains from a descriptor, tracks UTXOs and balance,
persists everything to SQLite, and can build, sign, and broadcast real transactions
against a live Bitcoin Core node.

## Setup

1. **A Bitcoin Core node running regtest**, reachable over RPC. For example:

   ```bash
   bitcoind -regtest -server -rpcuser=testuser -rpcpassword=testpass \
     -rpcbind=0.0.0.0 -rpcallowip=0.0.0.0/0 -fallbackfee=0.0002
   ```

   (Any regtest node works, including one started via Docker/Polar - just point
   `RPC_URL` at it.)

2. **Configure the wallet.** Copy `.env.example` to `.env` and fill in your node's RPC
   details. Nothing secret is hardcoded anywhere in this repo: the seed phrase and RPC
   credentials only ever live in `.env`, which is git-ignored.

   ```bash
   cp .env.example .env
   # edit .env: at minimum set RPC_URL / RPC_USER / RPC_PASS to match your node
   ```

3. **Initialize the wallet:**

   ```bash
   cargo run -- init
   ```

   If `.env` has no `MNEMONIC` set, this generates a fresh BIP-39 seed phrase, derives
   the external/internal descriptors from it, creates `wallet_data/wallet.sqlite`, and
   writes the mnemonic back into `.env` so every later command reuses the same wallet.

4. **Fund it (regtest has no faucet, so you mine your own coins):**

   ```bash
   cargo run -- mine --blocks 101   # matures the first coinbase reward
   cargo run -- sync
   cargo run -- balance
   ```

5. **Send a transaction:**

   ```bash
   cargo run -- address                                  # get a destination, or use any regtest address
   cargo run -- send --to <address> --amount 100000 --fee-rate 2
   cargo run -- mine --blocks 1                           # confirm it
   cargo run -- sync && cargo run -- balance
   ```

### Commands

| Command | Description |
|---|---|
| `init` | Generate/import keys, derive descriptors, create the wallet DB |
| `address` / `change-address` | Reveal a new external / internal address |
| `sync [--start-height N]` | Sync wallet state from the node |
| `balance [--sync]` | Print confirmed / pending / immature / total balance |
| `utxos` | List tracked UTXOs |
| `send --to <addr> --amount <sats> [--fee-rate <sat/vb>] [--utxo <txid:vout> ...]` | Build, sign, and broadcast a transaction |
| `mine --blocks <n> [--address <addr>]` | Regtest-only: mine blocks to fund the wallet |

Run `cargo run -- <command> --help` for full flag details.

### Proof of a working transaction

From an actual local regtest run of this wallet against `bitcoind` (see full session
transcript for context):

```
$ cargo run -- send --to bcrt1qv45cv9aqey7j0y00r7z4k8sxmsdnqfwgcu6prd --amount 500000 --fee-rate 2
broadcast txid: 30b8401e19fadd26615be6e75b8e6ce55a7cf474f4a377542917a643dd00b979

$ cargo run --example decode_raw_tx -- 30b8401e19fadd26615be6e75b8e6ce55a7cf474f4a377542917a643dd00b979
txid:     30b8401e19fadd26615be6e75b8e6ce55a7cf474f4a377542917a643dd00b979
version:  2
locktime: 102
weight:   561
input[0]
  previous_output: b05cefb794bf92a817b59965ebea1b72a0d83e0c52144b726a32da4ebd48b690:0
  sequence:        Sequence(0xfffffffd)
  witness items:   2
output[0]
  value:         0.00500000 BTC
  script type:   p2wpkh
output[1]
  value:         49.98499438 BTC
  script type:   p2wpkh
```

`bitcoin-cli -regtest getmempoolentry <txid>` confirmed the node accepted it into its
mempool immediately after broadcast, and after mining one more block and running
`sync`, `balance` reflected the new confirmed total. A second send using
`--utxo <txid:vout>` to force manual coin selection of a specific, matured coinbase
output was also broadcast successfully, and a send that named an immature coinbase
output was correctly rejected by the node with `bad-txns-premature-spend-of-coinbase`
and surfaced as a clean CLI error rather than a panic.

## Project / descriptor structure

```
src/
  main.rs      CLI definition (clap) and command dispatch
  config.rs    Loads all configuration from the environment / .env - no hardcoded secrets
  keys.rs      Mnemonic generation/import; derives wpkh/tr descriptors via BIP84/BIP86 paths
  node.rs      bitcoincore-rpc client setup, broadcast, regtest mining helper
  wallet.rs    Opens/creates the persisted BDK wallet; syncs it from the node
  commands.rs  One function per CLI subcommand, wiring the above together
  error.rs     A single AppError enum so every command returns Result instead of panicking
examples/
  decode_raw_tx.rs   Raw rust-bitcoin transaction decoding (see below)
```

Each keychain is derived at its own BIP44-style account path so external and internal
addresses can never collide: `m/84'/1'/0'/0/*` for receiving and `m/84'/1'/0'/1/*` for
change (or `m/86'/...` for the taproot variant). Keeping key derivation, node I/O, and
wallet state in separate modules mirrors how the three libraries are actually
responsible for different concerns - see the next section.

Descriptor kind is a config flag (`DESCRIPTOR_KIND=wpkh` or `taproot`) rather than a
compile-time choice, so both are exercised through the same code path. `wpkh` (native
segwit v0) is the default because it's simpler to sign and universally supported;
`tr` (taproot/BIP86) is offered as the stretch-goal comparison - same derivation
machinery, different script type and address prefix (`bcrt1q...` vs `bcrt1p...`).

## Library usage and why

- **`bdk_wallet`** owns all wallet-level state: descriptor parsing and key derivation
  (`descriptor!` macro + BIP-39), the external/internal keychain index, the UTXO/tx
  graph, balance calculation, transaction building (`TxBuilder`, coin selection), PSBT
  signing, and SQLite persistence (`rusqlite` feature). This is the layer where
  "wallet" logic belongs - it's exactly what BDK is for, and reimplementing UTXO
  tracking or coin selection by hand on top of raw `rust-bitcoin` would just be
  reinventing BDK.
- **`bitcoincore-rpc`** is the wallet's only connection to the outside world: connecting
  to `bitcoind`, health-checking it (`getblockchaininfo`), broadcasting the final signed
  transaction (`sendrawtransaction`), and mining regtest blocks for funding
  (`generatetoaddress`). Used directly rather than through BDK's wrapper because it's a
  thin, well-typed client and the assignment specifically asks for direct node
  integration via this crate.
- **`bdk_bitcoind_rpc`** bridges the two: its `Emitter` walks `bitcoind`'s active chain
  block-by-block via the RPC client and hands BDK connectable block/mempool events,
  which is how `sync` updates the wallet without BDK needing its own chain-sync logic
  or Electrum/Esplora dependency.
- **`rust-bitcoin`** (directly, not just through BDK's re-export) is used in
  `examples/decode_raw_tx.rs` for a case where BDK's `Wallet` API genuinely doesn't
  apply: decoding and inspecting an *arbitrary* raw transaction hex fetched via RPC
  (not one that touches this wallet's own descriptors). BDK's wallet has no concept of
  "decode a transaction I don't own" - that's a codec-level operation, so it's built
  straight on `bitcoin::consensus::deserialize` and `Transaction`/`Script` inspection,
  the same primitives this repo's Week 3 `decodetrx` lab was built on.
- **`clap`**, **`dotenvy`**, **`thiserror`**: CLI parsing, `.env` loading, and a single
  typed `AppError` so RPC/sqlite/config failures surface as readable error messages
  instead of panics.

## Known limitations / what I'd improve with more time

- `sync` is a simple blocking full rescan from `--start-height` (default: genesis) each
  time; it works fine on regtest but would be slow against a large testnet history. A
  persisted "last synced height" would make repeat syncs incremental.
- No RBF/CPFP support - `send` builds one transaction at a fixed fee rate with no bump
  path if it gets stuck.
- No multisig/miniscript policies beyond single-key `wpkh`/`tr` - would be a natural
  next step given BDK's policy compiler.
- The CLI is a single-shot process (each command opens and closes the DB); a
  long-running daemon mode with a background sync loop (like the upstream
  `bdk_bitcoind_rpc` example's threaded emitter) would give live balance updates
  instead of requiring an explicit `sync`.
- Only `wpkh` and `tr` are supported; no `sh(wpkh(...))` wrapped-segwit option for
  legacy-compatible receiving.
- No automated test suite: correctness was verified with real end-to-end runs against
  a live `bitcoind -regtest` (init → mine → sync → balance/utxos → send → confirm →
  restart-and-reload), documented above, rather than mocked unit tests. A follow-up
  would wrap that flow in a `#[test]` that spins up a throwaway regtest node.

## Constraints followed

- Regtest only by default (`testnet`/`signet` are also wired up via `BITCOIN_NETWORK`
  but untested here); `mine` explicitly refuses to run on anything but regtest.
- No private keys or seed phrases are committed: `.env` is git-ignored, `.env.example`
  ships with everything blank, and `init` only ever writes secrets into the
  git-ignored `.env`, never into source.
