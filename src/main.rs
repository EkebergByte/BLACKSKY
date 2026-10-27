mod config;
mod models;
mod rpc;

use crate::models::BlackskyError;
use crate::rpc::fetch_polygon_balance;

#[tokio::main]
async fn main() -> Result<(), BlackskyError> {
    dotenvy::dotenv().ok();
    let rpc_url = std::env::var("POLYGON_RPC_URL")
        .unwrap_or_else(|_| "https://polygon-bor-rpc.publicnode.com".to_string());
    let contract_address = "0xac0f66379a6d7801d7726d5a943356a172549adb";
    let wallet_address = "0x000000000000000000000000000000000000dEaD";

    let balance = fetch_polygon_balance(rpc_url, contract_address, wallet_address).await?;
    println!("Balance: {:.4} GEOD", balance);

    Ok(())
}
