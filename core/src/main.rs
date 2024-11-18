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
use core::ipc;

use crate::core::client::{run, Client};
use crate::utils::logger;
use anyhow::Result;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    logger::init_tracing();
    let (tx, rx) = mpsc::channel(100);
    //Used to send data back out through the socket
    let (data_tx, data_rx) = mpsc::channel(100);

    let runtime_tx = tx.clone();
    tokio::spawn(async move { ipc::listen(data_rx, runtime_tx, "7878".to_string()).await });

    let client = Client::new("./").await;
    run(client, tx, rx, data_tx).await?;
    Ok(())
}
