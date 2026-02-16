use std::time::Duration;

use hive_rs::api::BlockchainMode;
use hive_rs::{get_vests, Asset, Client, ClientOptions, HiveError, PrivateKey, PublicKey, Result};
use hive_rs::{Authority, Operation, TransferOperation};
use serde_json::json;

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(HiveError::Other(message.to_string()))
    }
}

fn with_step(step: &str, err: HiveError) -> HiveError {
    HiveError::Other(format!("{step}: {err}"))
}

#[derive(Debug, Clone)]
struct AuthConfig {
    username: String,
    active_key: String,
    broadcast_self_transfer: bool,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    load_dotenv();

    let nodes = target_nodes();
    let extended_checks = env_flag("HIVE_EXTENDED_CHECKS");

    let auth = auth_config_from_env();

    println!("Running hive-rs smoke test against {}", nodes.join(", "));
    match run(&nodes, auth.as_ref(), extended_checks).await {
        Ok(()) => {
            println!("Smoke test passed");
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Smoke test failed: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn load_dotenv() {
    let _ = dotenvy::from_filename_override("smoke-test-app/.env");
    let _ = dotenvy::dotenv_override();
}

fn auth_config_from_env() -> Option<AuthConfig> {
    let username = normalize_username(std::env::var("HIVE_USERNAME").ok()?.trim());
    let active_key = trim_wrapping_quotes(std::env::var("HIVE_ACTIVE_KEY").ok()?.trim());

    if username.trim().is_empty() || active_key.trim().is_empty() {
        return None;
    }

    Some(AuthConfig {
        username,
        active_key,
        broadcast_self_transfer: env_flag("HIVE_BROADCAST_SELF_TRANSFER"),
    })
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn normalize_username(value: &str) -> String {
    value.trim().trim_start_matches('@').to_string()
}

fn trim_wrapping_quotes(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn split_nodes(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|node| !node.is_empty())
        .map(|node| node.to_string())
        .collect()
}

fn target_nodes() -> Vec<String> {
    if let Some(arg_nodes) = std::env::args().nth(1).map(|value| split_nodes(&value)) {
        if !arg_nodes.is_empty() {
            return arg_nodes;
        }
    }

    if let Ok(nodes) = std::env::var("HIVE_NODES") {
        let parsed = split_nodes(&nodes);
        if !parsed.is_empty() {
            return parsed;
        }
    }

    if let Ok(node) = std::env::var("HIVE_NODE") {
        let parsed = split_nodes(&node);
        if !parsed.is_empty() {
            return parsed;
        }
    }

    vec![
        "https://api.hive.blog".to_string(),
        "https://api.openhive.network".to_string(),
    ]
}

async fn run(nodes: &[String], auth: Option<&AuthConfig>, extended_checks: bool) -> Result<()> {
    // Local crypto + key-format checks to ensure core primitives work before RPC.
    let private_key = PrivateKey::from_seed("hive-rs-smoke-seed")?;
    let public_key = private_key.public_key();
    let serialized_public = public_key.to_string();
    let parsed_public = PublicKey::from_string(&serialized_public)?;
    ensure(
        parsed_public.compressed_bytes() == public_key.compressed_bytes(),
        "public key round-trip mismatch",
    )?;

    let digest = [7_u8; 32];
    let signature = private_key.sign(&digest)?;
    ensure(
        public_key.verify(&digest, &signature),
        "signature verification failed",
    )?;

    let one_hive = Asset::from_string("1.000 HIVE")?;

    let mut options = ClientOptions::default();
    options.timeout = Duration::from_secs(15);
    let node_refs: Vec<&str> = nodes.iter().map(String::as_str).collect();
    let client = Client::new(node_refs, options);

    let props = client.database.get_dynamic_global_properties().await?;
    let account_count = client.database.get_account_count().await?;
    let latest_block = client
        .blockchain
        .get_current_block_num(BlockchainMode::Latest)
        .await?;

    ensure(
        latest_block >= props.last_irreversible_block_num,
        "latest block is lower than irreversible block",
    )?;

    let estimated_vests = get_vests(&props, &one_hive);

    println!(
        "head_block={}, irreversible_block={}, account_count={}",
        props.head_block_number, props.last_irreversible_block_num, account_count
    );
    println!("estimated_vests_for_1_hive={estimated_vests}");
    if extended_checks {
        run_extended_read_checks(&client).await?;
    } else {
        println!("extended_checks=skipped (set HIVE_EXTENDED_CHECKS=1 to enable)");
    }

    if let Some(auth) = auth {
        run_authenticated_checks(&client, auth, extended_checks).await?;
    } else {
        println!("authenticated_checks=skipped (set HIVE_USERNAME and HIVE_ACTIVE_KEY in .env)");
    }

    Ok(())
}

async fn run_extended_read_checks(client: &Client) -> Result<()> {
    let version = client.database.get_version().await?;
    let hardfork = client.database.get_hardfork_version().await?;
    let chain_props = client.database.get_chain_properties().await?;

    ensure(
        !version.blockchain_version.is_empty(),
        "version response did not include blockchain_version",
    )?;
    ensure(
        !hardfork.trim().is_empty(),
        "hardfork version response was empty",
    )?;
    ensure(
        chain_props.is_object(),
        "chain properties response was not an object",
    )?;

    println!(
        "extended_read_checks=ok blockchain_version={} hardfork={}",
        version.blockchain_version, hardfork
    );
    Ok(())
}

async fn run_authenticated_checks(
    client: &Client,
    auth: &AuthConfig,
    extended_checks: bool,
) -> Result<()> {
    println!("authenticated_checks=enabled for @{}", auth.username);

    let active_key = PrivateKey::from_wif(&auth.active_key)?;
    let active_public = active_key.public_key().to_string();
    println!("derived_active_public_key={active_public}");

    let active = match client
        .database
        .get_accounts(&[auth.username.as_str()])
        .await
    {
        Ok(accounts) => {
            ensure(!accounts.is_empty(), "account lookup returned no results")?;
            let account = &accounts[0];
            ensure(
                account.name == auth.username,
                "account lookup did not return requested username",
            )?;
            account
                .extra
                .get("active")
                .ok_or_else(|| {
                    HiveError::Other("account response missing active authority".to_string())
                })
                .and_then(|value| {
                    serde_json::from_value::<Authority>(value.clone())
                        .map_err(|err| HiveError::Serialization(err.to_string()))
                })?
        }
        Err(err) => {
            println!("database_get_accounts_typed=failed ({err}), trying raw rpc fallback");
            let raw_accounts: serde_json::Value = client
                .call(
                    "condenser_api",
                    "get_accounts",
                    json!([[auth.username.as_str()]]),
                )
                .await
                .map_err(|rpc_err| with_step("client.call(condenser_api.get_accounts)", rpc_err))?;

            let raw_account = raw_accounts
                .as_array()
                .and_then(|arr| arr.first())
                .ok_or_else(|| {
                    HiveError::Other("raw account lookup returned no results".to_string())
                })?;
            let raw_name = raw_account
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| HiveError::Other("raw account payload missing name".to_string()))?;
            ensure(
                raw_name == auth.username,
                "raw account lookup did not return requested username",
            )?;
            let active_value = raw_account.get("active").cloned().ok_or_else(|| {
                HiveError::Other("raw account payload missing active authority".to_string())
            })?;
            serde_json::from_value::<Authority>(active_value)
                .map_err(|parse_err| HiveError::Serialization(parse_err.to_string()))?
        }
    };

    let key_is_active = active
        .key_auths
        .iter()
        .any(|(key, weight)| key == &active_public && *weight > 0);
    ensure(
        key_is_active,
        "provided HIVE_ACTIVE_KEY is not present in account active authority",
    )?;

    let provided_key_weight = active
        .key_auths
        .iter()
        .find_map(|(key, weight)| {
            if key == &active_public {
                Some(*weight as u32)
            } else {
                None
            }
        })
        .unwrap_or(0);

    let key_refs = match client
        .keys
        .get_key_references(&[active_public.clone()])
        .await
    {
        Ok(refs) => {
            println!("key_reference_source=account_by_key_api");
            refs
        }
        Err(err) => {
            println!("key_reference_source=condenser_api_fallback ({err})");
            client
                .database
                .get_key_references(&[active_public.clone()])
                .await
                .map_err(|fallback_err| with_step("database.get_key_references", fallback_err))?
        }
    };
    let key_maps_to_account = key_refs
        .iter()
        .flat_map(|entry| entry.iter())
        .any(|name| name == &auth.username);
    ensure(
        key_maps_to_account,
        "account_by_key lookup did not map key to HIVE_USERNAME",
    )?;

    let transfer = TransferOperation {
        from: auth.username.clone(),
        to: auth.username.clone(),
        amount: Asset::from_string("0.001 HIVE")?,
        memo: "hive-rs auth smoke check".to_string(),
    };

    let tx = client
        .broadcast
        .create_transaction(vec![Operation::Transfer(transfer.clone())], None)
        .await
        .map_err(|err| with_step("broadcast.create_transaction", err))?;
    let tx_bytes = hive_rs::serialize_transaction(&tx)?;
    let tx_id = hive_rs::generate_trx_id(&tx)?;
    ensure(
        !tx_bytes.is_empty(),
        "serialized transaction bytes were empty",
    )?;
    ensure(
        tx_id.len() == 40,
        "generated transaction id had unexpected length",
    )?;

    let signed = client.broadcast.sign_transaction(&tx, &[&active_key])?;

    ensure(
        !signed.signatures.is_empty(),
        "signed transaction contains no signatures",
    )?;

    if provided_key_weight >= active.weight_threshold {
        let required = client
            .database
            .get_required_signatures(&signed, &[active_public.clone()])
            .await
            .map_err(|err| with_step("database.get_required_signatures", err))?;
        ensure(
            required.iter().any(|key| key == &active_public),
            "get_required_signatures did not include derived active key",
        )?;

        let authority_ok = client
            .database
            .verify_authority(&signed)
            .await
            .map_err(|err| with_step("database.verify_authority", err))?;
        ensure(authority_ok, "verify_authority returned false")?;
        println!("authority_check=full (provided key satisfies active threshold)");
    } else {
        println!(
            "authority_check=partial (provided key weight {} < threshold {})",
            provided_key_weight, active.weight_threshold
        );
    }

    if extended_checks {
        let rc_accounts = client
            .rc
            .find_rc_accounts(&[auth.username.as_str()])
            .await
            .map_err(|err| with_step("rc.find_rc_accounts", err))?;
        ensure(
            !rc_accounts.is_empty(),
            "rc.find_rc_accounts did not return account data",
        )?;
        let rc_cost = client
            .rc
            .calculate_cost(&[Operation::Transfer(transfer.clone())])
            .await
            .map_err(|err| with_step("rc.calculate_cost", err))?;
        println!(
            "extended_auth_checks=ok tx_id={} estimated_rc_cost={}",
            tx_id, rc_cost
        );
    } else {
        println!("extended_auth_checks=skipped (set HIVE_EXTENDED_CHECKS=1 to enable)");
    }

    if auth.broadcast_self_transfer {
        println!("broadcast_mode=async (condenser_api.broadcast_transaction)");
        let raw_broadcast_result: serde_json::Value = client
            .call("condenser_api", "broadcast_transaction", json!([signed]))
            .await
            .map_err(|err| with_step("client.call(condenser_api.broadcast_transaction)", err))?;
        println!("broadcast_async_result={raw_broadcast_result}");

        let mut status = "unknown".to_string();
        let mut used_condenser_fallback = false;
        for _ in 0..15 {
            if !used_condenser_fallback {
                match client.transaction.find_transaction(&tx_id).await {
                    Ok(found) => {
                        status = found.status;
                        if status != "unknown" {
                            break;
                        }
                    }
                    Err(HiveError::Rpc { message, .. })
                        if message.contains("Could not find method find_transaction") =>
                    {
                        used_condenser_fallback = true;
                        println!(
                            "transaction_status_source=condenser_api.get_transaction (fallback)"
                        );
                    }
                    Err(err) => {
                        println!("transaction_status_poll_error={err}");
                    }
                }
            }

            if used_condenser_fallback {
                match client
                    .call::<serde_json::Value>(
                        "condenser_api",
                        "get_transaction",
                        json!([tx_id.clone()]),
                    )
                    .await
                {
                    Ok(found) => {
                        let block_num = found
                            .get("block_num")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or_default();
                        if block_num > 0 {
                            status = "found_in_block".to_string();
                            break;
                        }
                    }
                    Err(HiveError::Rpc { message, .. })
                        if message.to_ascii_lowercase().contains("unknown transaction") =>
                    {
                        // Still propagating in mempool.
                    }
                    Err(err) => {
                        println!("transaction_status_poll_error={err}");
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        println!("broadcast_self_transfer_tx_id={tx_id}");
        println!("broadcast_self_transfer_status={status}");
        ensure(
            status != "unknown",
            "broadcast submitted but transaction status stayed unknown",
        )?;
    } else {
        println!("broadcast_self_transfer=skipped (set HIVE_BROADCAST_SELF_TRANSFER=1 to enable)");
    }

    Ok(())
}
