mod utils;

use discard::utils::types::TextMessage;
use tokio::sync::{mpsc, oneshot, Notify};
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

    let p1 = Client::new(test_paths[0]).await;
    let p2 = Client::new(test_paths[1]).await;
    let node2_id = p2.get_node_id();
    let message = String::from("test message");
    let message2 = message.clone();
    let p2_connected = Arc::new(Notify::new());

    let notify = Arc::clone(&p2_connected);
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        //peer 1 channel to simulate client running on their machine
        let (tx1, rx1) = mpsc::channel(10);
        let node_id = p1.get_node_id();
        let result = client::run(p1, tx1.clone(), rx1).await;
        assert!(result.is_ok());
        let result = tx1.send(RunMessage::ReceiveMessage).await;
        assert!(result.is_ok());
        let result = tx.send(node_id);
        assert!(result.is_ok());
    });

    tokio::spawn(async move {
        if let Ok(node_id) = rx.blocking_recv() {
            //peer 2 channel to simulate client running on their machine
            let (tx2, rx2) = mpsc::channel(10);
            let result = client::run(p2, tx2.clone(), rx2).await;
            assert!(result.is_ok());
            let text_message = TextMessage {
                content: "test".to_string(),
                timestamp: chrono::Utc::now(),
            };
            let result = tx2
                .send(RunMessage::SendMessage(node_id, text_message))
                .await;
            assert!(result.is_ok());
        };
    });
}
