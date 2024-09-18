mod utils {
    pub mod constants;
    pub mod debug;
    pub mod enums;
    pub mod errors;
    pub mod logger;
    pub mod types;
}
mod core {
    pub mod client;
    pub mod ipc;
    pub mod rtc;
    pub mod signal;
}
mod database {
    pub mod db;
    pub mod models;
}
use crate::core::client::{run, Client};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use utils::enums::RunMessage;

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel(100);
    //Used to send data back out through the socket
    let (data_tx, data_rx) = mpsc::channel(100);

    let listener_tx = tx.clone();
    tokio::spawn(async move {});

    let client = Client::new("./").await;
    run(client, tx, rx, data_tx).await?;
    Ok(())
}
