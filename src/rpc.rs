use reqwest::Client;
use serde_json::json;
use crate::models::BlackskyError;

pub async fn fetch_polygon_balance(
    rpc_url: &str,
    contract_address: &str,
    wallet_address: &str,
) -> Result<f64, BlackskyError> {
    let client = Client::new();
    let cleaned_address = wallet_address.trim_start_matches("0x");
    let padded_address = format!("{:0>64}", cleaned_address);
    let data = format!("0x70a08231{}", padded_address);

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

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| BlackskyError::NetworkError(e.to_string()))?;

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
