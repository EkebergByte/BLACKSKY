mod config;
mod models;
mod rpc;

use crate::models::BlackskyError;
use crate::rpc::fetch_polygon_balances_batch;
use crate::config::{TRACKED_ENTITIES, Chain};

#[tokio::main]
async fn main() -> Result<(), BlackskyError> {
    dotenvy::dotenv().ok();
    let rpc_url = "https://1rpc.io/matic".to_string();
    let contract_address = "0xac0f66379a6d7801d7726d5a943356a172549adb";

    let polygon_addresses: Vec<&str> = TRACKED_ENTITIES
        .iter()
        .filter(|e| e.chain == Chain::Polygon)
        .map(|e| e.address)
        .collect();

    let balances = fetch_polygon_balances_batch(&rpc_url, contract_address, &polygon_addresses).await?;

    for entity in TRACKED_ENTITIES.iter().filter(|e| e.chain == Chain::Polygon) {
        if let Some((_, balance)) = balances.iter().find(|(addr, _)| *addr == entity.address) {
            println!("{} ({}) - {:.4} GEOD", entity.label, entity.address, balance);
        }
    }

    Ok(())
}
