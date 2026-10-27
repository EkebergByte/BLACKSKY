use reqwest::Client;
use serde_json::json;
use crate::models::BlackskyError;

pub async fn fetch_polygon_balance(
    rpc_url: &str,
    contract_address: &str,
    wallet_address: &str,
) -> Result<f64, BlackskyError> {
    let client = Client::builder()
        .user_agent("blacksky/0.1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| BlackskyError::NetworkError(e.to_string()))?;

    let cleaned_address = wallet_address
        .trim_start_matches("0x")
        .to_lowercase();
    if cleaned_address.len() != 40 {
        return Err(BlackskyError::HexDecodeError("Invalid address length".to_string()));
    }
    let padded_address = format!("{:0>64}", cleaned_address);
    let data = format!("0x70a08231{}", padded_address);
    if data.len() != 74 {
        return Err(BlackskyError::HexDecodeError("Invalid ABI data length".to_string()));
    }

    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": contract_address,
            "data": data
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

    if result_hex.is_empty() {
        return Ok(0.0);
    }

    let balance_u128 = u128::from_str_radix(result_hex, 16)
        .map_err(|e| BlackskyError::HexDecodeError(e.to_string()))?;

    Ok(balance_u128 as f64 / 1e18)
}
