mod utils;

use tokio::sync::{mpsc, Notify};
use tracing::{error, info};

use std::sync::Arc;

use discard::core::client::{self, Client};
use discard::utils::enums::RunMessage;
use discard::utils::logger;
use utils::Cleanup;

//Create two clients and test sdp exchange via sending store bytes from iroh
#[tokio::test]
async fn test_data_channel() {
    logger::init_tracing();
    let test_paths = vec!["./test-root1", "./test-root2", "./test-db.db3"];

    //Will remove test paths again at the end of the test
    let cleanup = Cleanup {
        test_paths: &test_paths,
    };
    cleanup.remove_test_paths();

    let mut p1 = Client::new(test_paths[0]).await;
    let mut p2 = Client::new(test_paths[1]).await;
    let node2_id = p2.get_node_id();
    let message = String::from("test message");
    let message2 = message.clone();
    let p2_connected = Arc::new(Notify::new());

    let notify = Arc::clone(&p2_connected);

    //peer 1 channel to simulate client running on their machine
    let (tx1, rx1) = mpsc::channel(10);
    //peer 2 channel to simulate client running on their machine
    let (tx2, rx2) = mpsc::channel(10);

    let tx1_clone = tx1.clone();
    let tx2_clone = tx2.clone();

    tokio::spawn(async move {
        let result = client::run(p1, tx1_clone, rx1).await;
    });
    tokio::spawn(async { client::run(p2, tx2_clone, rx2).await });
    let result = tx1.send(RunMessage::ReceiveMessage).await;
    assert!(result
        .map_err(|e| println!("Failed to receive message. {}", e))
        .is_ok())
    //Receive conncetion from p1 by listening for new Sessions + p1.node_id
}
