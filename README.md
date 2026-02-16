# hive-rs

A Rust client library for the Hive blockchain with API coverage modeled after `dhive`.

This crate gives you:

- Strongly typed request/response APIs for common Hive namespaces
- Signing and transaction serialization utilities
- Multi-node failover transport
- Helpers for RC estimation, assets, keys, memo encryption, and block streaming

## Table Of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Client Configuration](#client-configuration)
- [API Modules](#api-modules)
- [Transactions And Signing](#transactions-and-signing)
- [Streaming Blocks And Operations](#streaming-blocks-and-operations)
- [Raw RPC Calls](#raw-rpc-calls)
- [Errors And Reliability](#errors-and-reliability)
- [Security Notes](#security-notes)
- [Smoke Test App](#smoke-test-app)
- [Development](#development)

## Installation

```toml
[dependencies]
hive-rs = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### TLS Features

- Default: `rustls`
- Optional: `native-tls`

```toml
[dependencies]
hive-rs = { version = "0.1", default-features = false, features = ["native-tls"] }
```

### Network Feature

- `testnet`: switches default chain id in `ClientOptions::default()`

## Quick Start

```rust
use hive_rs::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new_default();

    let props = client.database.get_dynamic_global_properties().await?;
    let account_count = client.database.get_account_count().await?;

    println!("head block: {}", props.head_block_number);
    println!("account count: {}", account_count);

    Ok(())
}
```

## Client Configuration

Use one or more node URLs. The transport rotates across nodes for retryable transport failures.

```rust
use std::time::Duration;

use hive_rs::transport::BackoffStrategy;
use hive_rs::{Client, ClientOptions, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let options = ClientOptions {
        timeout: Duration::from_secs(15),
        failover_threshold: 2,
        backoff: BackoffStrategy::Exponential {
            base_ms: 100,
            max_ms: 5_000,
        },
        ..ClientOptions::default()
    };

    let client = Client::new(
        vec![
            "https://api.hive.blog",
            "https://api.openhive.network",
        ],
        options,
    );

    let version = client.database.get_version().await?;
    println!("chain version: {}", version.blockchain_version);

    Ok(())
}
```

## API Modules

`Client` exposes namespace clients directly:

- `client.database` (`condenser_api`)
- `client.broadcast` (`condenser_api` broadcast helpers)
- `client.blockchain` (block/head helpers + streams)
- `client.hivemind` (`bridge` methods)
- `client.rc` (`rc_api` methods + RC cost estimator)
- `client.keys` (`account_by_key_api` with legacy fallback)
- `client.transaction` (`transaction_status_api` with condenser fallback)

Example combining account lookup + key references:

```rust
use hive_rs::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new_default();

    let accounts = client.database.get_accounts(&["beggars"]).await?;
    println!("loaded {} account(s)", accounts.len());

    let refs = client
        .keys
        .get_key_references(&["STM5UaWzkk9yCBdAANGp24Tnn3ecibgLTmWCQJuJtzVcdNLAW9fQn".to_string()])
        .await?;

    println!("key refs: {:?}", refs);

    Ok(())
}
```

## Transactions And Signing

### High-level transfer

```rust
use hive_rs::{Asset, Client, PrivateKey, Result, TransferOperation};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new_default();
    let active_key = PrivateKey::from_wif("<YOUR_ACTIVE_WIF>")?;

    let op = TransferOperation {
        from: "alice".to_string(),
        to: "bob".to_string(),
        amount: Asset::from_string("0.001 HIVE")?,
        memo: "hive-rs test".to_string(),
    };

    let confirmation = client.broadcast.transfer(op, &active_key).await?;
    println!(
        "tx id={} block_num={} trx_num={}",
        confirmation.id, confirmation.block_num, confirmation.trx_num
    );

    Ok(())
}
```

### Build/sign manually

```rust
use hive_rs::{
    generate_trx_id, serialize_transaction, Asset, Client, Operation, PrivateKey, Result,
    TransferOperation,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new_default();
    let active_key = PrivateKey::from_wif("<YOUR_ACTIVE_WIF>")?;

    let op = Operation::Transfer(TransferOperation {
        from: "alice".to_string(),
        to: "alice".to_string(),
        amount: Asset::from_string("0.001 HIVE")?,
        memo: "manual tx".to_string(),
    });

    let tx = client.broadcast.create_transaction(vec![op], None).await?;
    let tx_id = generate_trx_id(&tx)?;
    let tx_bytes = serialize_transaction(&tx)?;
    let signed = client.broadcast.sign_transaction(&tx, &[&active_key])?;

    println!("tx id: {}", tx_id);
    println!("serialized bytes: {}", tx_bytes.len());

    let confirmation = client.broadcast.send(signed).await?;
    println!("broadcasted id: {}", confirmation.id);

    Ok(())
}
```

## Streaming Blocks And Operations

```rust
use futures::StreamExt;
use hive_rs::api::{BlockchainMode, BlockchainStreamOptions};
use hive_rs::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new_default();

    let stream = client.blockchain.get_block_numbers(BlockchainStreamOptions {
        from: None,
        to: Some(5),
        mode: BlockchainMode::Latest,
    });

    futures::pin_mut!(stream);
    while let Some(block_num) = stream.next().await {
        println!("block={}", block_num?);
    }

    Ok(())
}
```

## Raw RPC Calls

If you need a method that is not wrapped yet, use `Client::call`.

```rust
use serde_json::json;

use hive_rs::{Client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new_default();

    let config: serde_json::Value = client
        .call("condenser_api", "get_config", json!([]))
        .await?;

    println!("config keys: {}", config.as_object().map(|m| m.len()).unwrap_or(0));

    Ok(())
}
```

## Errors And Reliability

The crate uses `HiveError` for all fallible operations.

Common variants:

- `HiveError::Rpc { code, message, data }`
- `HiveError::Transport(...)`
- `HiveError::Timeout`
- `HiveError::Serialization(...)`
- `HiveError::AllNodesFailed`

Reliability behavior:

- Failover retries only retryable transport failures (not RPC/serialization logic errors).
- `AccountByKeyApi::get_key_references` falls back to `condenser_api.get_key_references` when appbase format is unsupported by a node.
- `TransactionStatusApi::find_transaction` falls back to `condenser_api.get_transaction` when `transaction_status_api` is unavailable.
- `BroadcastApi::send` attempts synchronous broadcast first, then falls back to async broadcast plus transaction lookup for confirmation on nodes that do not support reliable synchronous responses.

## Security Notes

- Never commit private keys.
- Use environment variables or a secrets manager for key material.
- Prefer dedicated active/posting keys with least privilege.
- Consider running your own trusted node for production traffic.

## Smoke Test App

A runnable smoke-test client lives in `smoke-test-app/` and can validate both read and authenticated flows.

```bash
cargo run --manifest-path smoke-test-app/Cargo.toml
```

For authenticated checks, create `smoke-test-app/.env` (already gitignored):

```env
HIVE_NODE=https://api.hive.blog
HIVE_USERNAME=beggars
HIVE_ACTIVE_KEY=<ACTIVE_WIF>
HIVE_EXTENDED_CHECKS=1
HIVE_BROADCAST_SELF_TRANSFER=0
```

Set `HIVE_BROADCAST_SELF_TRANSFER=1` to submit a real self-transfer test transaction.

## Development

```bash
cargo fmt
cargo test
```

Current status:

- Unit tests cover serialization, crypto, API routing, failover behavior, fallback behavior, and RC calculations.
- The smoke-test app validates real-node integration for end-to-end sanity checks.
