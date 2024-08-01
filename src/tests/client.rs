use std::path::{self, PathBuf};

use tokio::fs;
use tracing::{debug, info, warn};
use tracing_test::traced_test;

use crate::core::client::Client;
const TEST_PATHS: [&str; 2] = ["./test-root1", "./test-root2"];
//Create two clients and test sdp exchange via sending store bytes from iroh
/*#[tokio::test]
#[traced_test]
async fn test_connection() {
    info!("TEST");
    remove_test_paths();
    let mut p1 = Client::new(TEST_PATHS[0]).await;
    info!("Created client 1");
    let p2 = Client::new(TEST_PATHS[1]).await;
    info!("Created client 2");
    let node2_id = p2.get_node_id();
    //Initiliaze the connection with p2 by sending Session + self.node_id
    if let Err(e) = p1.init_connection(node2_id).await {
        panic!("Error initializing the conn {e}");
    };
    //Receive conncetion from p1 by listening for new Sessions + p1.node_id
    if let Err(e) = p2.receive_connection().await {
        panic!("Error receiving the conn {e}");
    }
}*/

#[tokio::test]
#[traced_test]
async fn test_client_creation() {
    remove_test_paths().await;
    let client = Client::new(TEST_PATHS[0]).await;
    info!("Created Client {}", client.get_node_id());
}

async fn remove_test_paths() {
    for path in TEST_PATHS {
        if PathBuf::from(path).exists() {
            match fs::remove_dir_all(path).await {
                Ok(_) => info!("Removed test path: {:?}", path),
                Err(e) => warn!("Failed to remove test paths: {:?} {}", path, e),
            }
        }
    }
}
