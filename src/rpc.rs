use reqwest::Client;
use serde_json::json;
use crate::models::BlackskyError;

pub async fn fetch_polygon_balances_batch(
    rpc_url: &str,
    contract_address: &str,
    wallet_addresses: &[&str],
) -> Result<Vec<(String, f64)>, BlackskyError> {
    let client = Client::builder()
        .user_agent("blacksky/0.1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| BlackskyError::NetworkError(e.to_string()))?;

    let cleaned_contract = contract_address
        .trim_start_matches("0x")
        .to_lowercase();
    if cleaned_contract.len() != 40 {
        return Err(BlackskyError::HexDecodeError("Invalid contract address length".to_string()));
    }

    let n = wallet_addresses.len();
    let mut calldata = String::from("0x252dba42");
    calldata.push_str(&format!("{:064x}", 32));
    calldata.push_str(&format!("{:064x}", n));

    for i in 0..n {
        calldata.push_str(&format!("{:064x}", 32 * n + 160 * i));
    }

    for wallet_address in wallet_addresses {
        let cleaned_wallet = wallet_address
            .trim_start_matches("0x")
            .to_lowercase();
        if cleaned_wallet.len() != 40 {
            return Err(BlackskyError::HexDecodeError("Invalid wallet address length".to_string()));
        }

        calldata.push_str(&format!("{:0>64}", cleaned_contract));
        calldata.push_str("0000000000000000000000000000000000000000000000000000000000000040");
        calldata.push_str("0000000000000000000000000000000000000000000000000000000000000024");
        calldata.push_str(&format!("70a08231000000000000000000000000{}00000000000000000000000000000000000000000000000000000000", cleaned_wallet));
    }

    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": "0xcA11bde05977b3631167028862bE2a173976CA11",
            "data": calldata
        }, "latest"],
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| BlackskyError::NetworkError(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(BlackskyError::NetworkError(format!("HTTP {}: {}", status, body)));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BlackskyError::NetworkError(e.to_string()))?;

    if let Some(error) = response_json.get("error") {
        return Err(BlackskyError::RpcResponseError(format!(
            "RPC error: {}",
            error["message"].as_str().unwrap_or("Unknown error")
        )));
    }

    let result_hex = response_json["result"]
        .as_str()
        .ok_or_else(|| BlackskyError::RpcResponseError("Missing result field".to_string()))?
        .trim_start_matches("0x");

    if result_hex.len() < 192 + 192 * n {
        return Err(BlackskyError::HexDecodeError("Invalid response length".to_string()));
    }

    let mut results = Vec::new();
    for i in 0..n {
        let start = 192 + 64 * n + 128 * i + 64;
        let balance_word = &result_hex[start..start + 64];
        let balance_u128 = u128::from_str_radix(&balance_word[32..64], 16)
            .map_err(|e| BlackskyError::HexDecodeError(e.to_string()))?;
        results.push((wallet_addresses[i].to_string(), balance_u128 as f64 / 1e18));
    }

    Ok(results)
}
