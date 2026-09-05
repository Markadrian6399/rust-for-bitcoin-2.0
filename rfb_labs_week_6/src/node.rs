use bitcoincore_rpc::{Auth, Client, RpcApi};

use crate::config::Config;
use crate::error::{AppError, AppResult};

/// Build a `bitcoincore-rpc` client from configuration, preferring cookie-file auth
/// when available (the default for a freshly started `bitcoind -regtest`) and falling
/// back to user/pass, matching how Bitcoin Core itself is normally accessed locally.
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
    client.get_blockchain_info().map_err(|e| {
        AppError::Config(format!(
            "could not reach bitcoind at {} ({e}); is it running with `-regtest -server`?",
            config.rpc_url
        ))
    })?;

    Ok(client)
}

/// Broadcast a signed transaction through the connected node.
pub fn broadcast(
    client: &Client,
    tx: &bdk_wallet::bitcoin::Transaction,
) -> AppResult<bdk_wallet::bitcoin::Txid> {
    Ok(client.send_raw_transaction(tx)?)
}

/// Regtest-only convenience: mine `n` blocks paying the coinbase to `address`. There is
/// no faucet on regtest, so this is how the wallet gets its own test coins.
pub fn mine_to_address(
    client: &Client,
    n: u64,
    address: &bdk_wallet::bitcoin::Address,
) -> AppResult<Vec<bdk_wallet::bitcoin::BlockHash>> {
    Ok(client.generate_to_address(n, address)?)
}
