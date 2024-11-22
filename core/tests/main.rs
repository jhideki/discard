mod utils;

use discard::core::{client, ipc};
use discard::utils::logger;
use utils::Cleanup;

use anyhow::Result;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_runtime() {
    //Test setup/cleanup
    let test_paths = vec!["./test_path"];

    logger::init_tracing();
    let cleanup = Cleanup {
        test_paths: &test_paths,
    };

    cleanup.remove_test_paths();

    let (tx, rx) = mpsc::channel(100);
    //Used to send data back out through the socket
    let (data_tx, data_rx) = mpsc::channel(100);

    let runtime_tx = tx.clone();
    tokio::spawn(async move { ipc::listen(data_rx, runtime_tx, "7878".to_string()).await });

    let client = client::Client::new("./").await;
    client::run(client, tx, rx, data_tx)
        .await
        .expect("Failed to run client");
}
