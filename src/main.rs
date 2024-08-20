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
    pub mod rtc;
    pub mod signal;
}
mod database {
    pub mod db;
    pub mod models;
}
use crate::core::client::{run, Client};
use anyhow::Result;
use tokio::sync::mpsc;
#[tokio::main]
async fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel(100);
    tokio::spawn(async {
        let client = Client::new("./").await;
        run(client, tx, rx).await;
    });
    Ok(())
}
