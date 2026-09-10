// ============================================================================
// This file is our telephone to the "big shared notebook keeper" - the
// Bitcoin node (bitcoind). The node knows about EVERY coin and EVERY note
// (transaction) that's ever happened. We call it up to:
//   - say hello and make sure it's awake (`connect`)
//   - mail it our signed notes so the whole world can see them (`broadcast`)
//   - (on our practice playground only) ask it to make pretend money appear
//     so we have something to play with (`mine_to_address`)
// ============================================================================

use bitcoincore_rpc::{Auth, Client, RpcApi};

use crate::config::Config;
use crate::error::{AppError, AppResult};

/// Build a `bitcoincore-rpc` client from configuration, preferring cookie-file auth
/// when available (the default for a freshly started `bitcoind -regtest`) and falling
/// back to user/pass, matching how Bitcoin Core itself is normally accessed locally.
// Dial the phone number (RPC_URL) and figure out the secret knock (auth)
// that proves we're allowed to talk to this notebook keeper.
pub fn connect(config: &Config) -> AppResult<Client> {
    let auth = match (&config.rpc_cookie, &config.rpc_user, &config.rpc_pass) {
        (Some(cookie), _, _) => Auth::CookieFile(cookie.clone()),
        (None, Some(user), Some(pass)) => Auth::UserPass(user.clone(), pass.clone()),
        (None, None, None) => Auth::None,
        _ => {
            return Err(AppError::Config(
                "set both RPC_USER and RPC_PASS, or RPC_COOKIE, to authenticate with bitcoind"
                    .into(),
            ))
        }
    };

    let client = Client::new(&config.rpc_url, auth)?;
    // Fail fast with a clear error if the node isn't reachable, instead of surfacing a
    // confusing error later during sync or broadcast.
    // Say "knock knock, are you there?" right away, so if nobody answers we
    // tell you immediately instead of confusing you later.
    client.get_blockchain_info().map_err(|e| {
        AppError::Config(format!(
            "could not reach bitcoind at {} ({e}); is it running with `-regtest -server`?",
            config.rpc_url
        ))
    })?;

    Ok(client)
}

/// Broadcast a signed transaction through the connected node.
// Hand our finished, stamped note to the notebook keeper and say "please
// tell everyone about this!"
pub fn broadcast(
    client: &Client,
    tx: &bdk_wallet::bitcoin::Transaction,
) -> AppResult<bdk_wallet::bitcoin::Txid> {
    Ok(client.send_raw_transaction(tx)?)
}

/// Regtest-only convenience: mine `n` blocks paying the coinbase to `address`. There is
/// no faucet on regtest, so this is how the wallet gets its own test coins.
// Press the "make pretend money" button `n` times. Each press creates one
// new block (a new page in the notebook) and gives a fresh batch of
// practice coins to whichever address we point at.
pub fn mine_to_address(
    client: &Client,
    n: u64,
    address: &bdk_wallet::bitcoin::Address,
) -> AppResult<Vec<bdk_wallet::bitcoin::BlockHash>> {
    Ok(client.generate_to_address(n, address)?)
}
