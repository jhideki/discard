mod utils;

use tokio::sync::Notify;
use tracing::{error, info};

use std::sync::Arc;

use discard::core::client::Client;
use discard::utils::enums::MessageType;
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

    //Receive conncetion from p1 by listening for new Sessions + p1.node_id
    let handle = tokio::spawn(async move {
        let result = p2.receive_connection().await;

        notify.notify_one();

        assert!(result.is_ok());

        let mut receivers = result.expect("error");

        if let Some(MessageType::String(received_message)) = receivers[0].recv().await {
            info!("Received message: {}", received_message);

            assert!(received_message == message2, "Messages are not equal");
        }
    });

    //Initiliaze the connection with p2 by sending Session + self.node_id
    let result = p1.init_connection(node2_id).await;
    assert!(result
        .map_err(|e| println!("Failed initiliaze connection:  {}", e))
        .is_ok());

    println!("Waiting...");
    p2_connected.notified().await;

    let result = p1.send_message(0, message.clone()).await;
    println!("Sent message");
    assert!(result
        .map_err(|e| println!("Failed to send message: {}", e))
        .is_ok());

    let result = handle.await;
    assert!(result
        .map_err(|e| println!("Failed to jion handle: {}", e))
        .is_ok());
}
