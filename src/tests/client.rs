use std::path::PathBuf;

use tokio::fs;
use tracing::{error, info, warn};

use crate::core::client::Client;
use crate::utils::logger;

//Create two clients and test sdp exchange via sending store bytes from iroh
#[tokio::test]
async fn test_data_channel() {
    logger::init_signal_file_trace();
    let test_paths = vec!["./test-root1", "./test-root2"];
    remove_test_paths(&test_paths).await;

    let mut p1 = Client::new(test_paths[0]).await;
    let p2 = Client::new(test_paths[1]).await;
    let node2_id = p2.get_node_id();
    let message = String::from("test message");
    let message2 = message.clone();

    //Receive conncetion from p1 by listening for new Sessions + p1.node_id
    tokio::spawn(async move {
        if let Err(e) = p2.receive_connection().await {
            error!("Error receiving the conn {e}");
        }

        let result = p2.get_message(0).await;
        assert!(result.is_ok());
        if let Ok(received_message) = result {
            info!("Message: {}", received_message);
            assert!(received_message == message2, "Messages are not equal");
        } else {
            error!("No message received");
        }
    });

    //Initiliaze the connection with p2 by sending Session + self.node_id
    if let Err(e) = p1.init_connection(node2_id).await {
        error!("Error initializing the conn {e}");
    };
    let result = p1.send_message(0, message.clone()).await;
    assert!(result.is_ok());

    //cleanup
    remove_test_paths(&test_paths).await;
}

#[tokio::test]
async fn test_node_id_exchange() {
    logger::init_tracing();
    let test_paths = vec!["./test-root3", "./test-root4"];
    remove_test_paths(&test_paths).await;
    info!("Removed test paths");

    let p1 = Client::new(test_paths[0]).await;
    let p2 = Client::new(test_paths[1]).await;
    let node2_id = p2.get_node_id();
    let node1_id = p1.get_node_id().clone();

    //Set up listner
    tokio::spawn(async move {
        if let Ok(recieved_id) = p2.get_remote_node_id().await {
            assert!(
                recieved_id.fmt_short() == node1_id.fmt_short(),
                "Node ids do not match"
            );
        }
    });

    let _ = p1.send_remote_node_id(node2_id).await;

    //cleanup
    remove_test_paths(&test_paths).await;
}

async fn remove_test_paths(test_paths: &Vec<&str>) {
    for path in test_paths {
        if PathBuf::from(path).exists() {
            match fs::remove_dir_all(path).await {
                Ok(_) => info!("Removed test path: {:?}", path),
                Err(e) => warn!("Failed to remove test paths: {:?} {}", path, e),
            }
        }
    }
}
