mod utils;

use discard::core::client::{run, Client};
use discard::core::ipc::{self, IPCMessage, IPCResponse, SendMessageMsg, SendUsersResp};
use discard::utils::enums::RunMessage;
use discard::utils::logger;
use iroh::net::key::PublicKey;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use utils::Cleanup;

use std::collections::HashSet;

async fn ipc_setup() -> (TcpStream, Cleanup) {
    //Test setup/cleanup
    let test_paths = vec!["./test_ipc_get_messages".to_string()];

    logger::init_tracing();

    //Send RunMessage
    let (runmessage_tx, runmessage_rx) = mpsc::channel(100);
    //Used to receive and send data back out through the socket
    let (data_tx, data_rx) = mpsc::channel(100);

    let cleanup = Cleanup {
        test_paths: test_paths.clone(),
        runmessage_tx: runmessage_tx.clone(),
    };
    cleanup.remove_test_paths();

    //Spawn ipc handler
    let runtime_tx = runmessage_tx.clone();
    tokio::spawn(async move { ipc::listen(data_rx, runtime_tx, "7878".to_string()).await });

    let client = Client::new(&test_paths[0]).await;

    tokio::spawn(async move {
        run(client, runmessage_tx, runmessage_rx, data_tx)
            .await
            .expect("Failed to run client");
    });

    let result = timeout(Duration::from_secs(10), async {
        loop {
            match TcpStream::connect("127.0.0.0:7878").await {
                Ok(result) => {
                    return result;
                }
                Err(_) => {
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    })
    .await;

    assert!(result.is_ok(), "Timeout error in setup");
    (result.unwrap(), cleanup)
}

#[tokio::test]
async fn test_ipc_get_messages() {}

#[tokio::test]
async fn test_ipc_add_user() {
    let (mut stream, cleanup) = ipc_setup().await;

    let test_key = PublicKey::from_bytes(&[0; 32]).expect("Error generating test key");
    let message = RunMessage::Adduser(test_key, "test_user1".to_string());
    let bytes = serde_json::to_vec(&message).expect("Error serializing to bytes");
    let num_bytes = stream.write(&bytes).await.expect("Error writing to stream");
    println!("Wrote {}: ", num_bytes);

    let message = RunMessage::GetUser("test_user1".to_string());
    let bytes = serde_json::to_vec(&message).expect("Error serializing to bytes");
    let mut buf = vec![0; 1024];
    stream
        .read(&mut buf)
        .await
        .expect("Error reading into buffer");
    let response: IPCResponse = serde_json::from_slice(&buf).expect("Error deserializing response");
    if let IPCResponse::SendUser(user) = response {
        assert!(user.display_name == "test_user1".to_string());
        assert!(user.node_id == test_key.to_string());
    } else {
        panic!("Error deserializing ipc respons into user struct")
    }
}
