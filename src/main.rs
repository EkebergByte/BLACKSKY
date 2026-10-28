mod config;
mod models;
mod rpc;
mod storage;

use chrono::Utc;
use crate::models::{BlackskyError, SnapshotRecord};
use crate::rpc::fetch_polygon_balances_batch;
use crate::config::{TRACKED_ENTITIES, Chain};
use crate::storage::append_snapshot_records;

#[tokio::main]
async fn main() -> Result<(), BlackskyError> {
    dotenvy::dotenv().ok();
    let rpc_url = "https://1rpc.io/matic".to_string();
    let contract_address = "0xac0f66379a6d7801d7726d5a943356a172549adb";

    let now = Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let timestamp_utc = now.timestamp();

    let polygon_addresses: Vec<&str> = TRACKED_ENTITIES
        .iter()
        .filter(|e| e.chain == Chain::Polygon)
        .map(|e| e.address)
        .collect();

    let (block_number, balances) = fetch_polygon_balances_batch(&rpc_url, contract_address, &polygon_addresses).await?;

    let mut records: Vec<SnapshotRecord> = TRACKED_ENTITIES
        .iter()
        .filter(|e| e.chain == Chain::Polygon)
        .filter_map(|entity| {
            balances.iter()
                .find(|(addr, _)| *addr == entity.address)
                .map(|(_, balance)| SnapshotRecord {
                    date: date.clone(),
                    timestamp_utc,
                    chain: entity.chain.as_str().to_string(),
                    block_number,
                    address: entity.address.to_string(),
                    category: entity.category.as_str().to_string(),
                    label: entity.label.to_string(),
                    is_liquid_overhang: entity.is_liquid_overhang,
                    balance_geod: *balance,
                })
        })
        .collect();

    records.sort_by(|a, b| b.balance_geod.partial_cmp(&a.balance_geod).unwrap());

    println!("{:<8} | {:<20} | {:<38} | {:<42} | {:>18} | {:<8}", "CHAIN", "CATEGORY", "LABEL", "ADDRESS", "BALANCE (GEOD)", "OVERHANG");
    println!("{}", "-".repeat(149));

    let mut frozen_reserve = 0.0;
    let mut burned_total = 0.0;
    let mut liquid_overhang = 0.0;

    for record in &records {
        println!("{:<8} | {:<20} | {:<38} | {:<42} | {:>18.4} | {:<8}",
            record.chain,
            record.category,
            record.label,
            record.address,
            record.balance_geod,
            record.is_liquid_overhang);

        match record.category.as_str() {
            "ProtocolReserve" | "TeamAllocationVault" | "PublicSaleVault" => frozen_reserve += record.balance_geod,
            "BurnAddress" => burned_total += record.balance_geod,
            _ if record.is_liquid_overhang => liquid_overhang += record.balance_geod,
            _ => {}
        }
    }

    let inserted_count = append_snapshot_records("data/snapshots.csv", &records)?;

    println!("\nSummary:");
    println!("Polygon Block Number: {}", block_number);
    println!("Frozen Reserve: {:.4} GEOD", frozen_reserve);
    println!("Burned Total: {:.4} GEOD", burned_total);
    println!("Liquid Overhang: {:.4} GEOD", liquid_overhang);
    println!("New Records Written: {}", inserted_count);

    Ok(())
}
