mod utils;

use anyhow::Result;
use discard::core::client::{run, Client};
use discard::core::ipc::{self, IPCMessage, IPCResponse, SendMessageMsg};
use discard::utils::logger;
use iroh::net::key::PublicKey;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use utils::Cleanup;

#[tokio::test]
async fn test_ipc_get_messages() {
    //Test setup/cleanup
    let test_paths = vec!["./test_path"];

    logger::init_tracing();
    let cleanup = Cleanup {
        test_paths: &test_paths,
    };

    cleanup.remove_test_paths();
    //Send RunMessage
    let (runmessage_tx, mut runmessage_rx) = mpsc::channel(100);
    //Used to receive and send data back out through the socket
    let (data_tx, mut data_rx) = mpsc::channel(100);

    //Spawn ipc handler
    let runtime_tx = runmessage_tx.clone();
    tokio::spawn(async move { ipc::listen(data_rx, runtime_tx, "7878".to_string()).await });

    let mut client = Client::new(test_paths[0]).await;
    let key = PublicKey::from_bytes(&[0; 32]).expect("Failed to gen dummy key");
    client
        .add_user(key, "test_user1".to_string())
        .expect("Failed to add dummy user");

    client
        .add_user(key, "test_user2".to_string())
        .expect("Failed to add dummy user");

    tokio::spawn(async move {
        run(client, runmessage_tx, runmessage_rx, data_tx).await;
    });

    let result = match timeout(Duration::from_secs(5), async {
        loop {
            let result = TcpStream::connect("127.0.0.1:7878").await;
            if result.is_ok() {
                println!("Connected on 127.0.0.1:7878");
                return result;
            }
            sleep(Duration::from_secs(1)).await;
        }
    })
    .await
    {
        Ok(stream) => stream,
        Err(e) => panic!("Failed to connect to tcp stream {}", e),
    };
    let mut stream = result.unwrap();

    let get_users_msg = IPCMessage::GetUsers;
    let bytes = serde_json::to_vec(&get_users_msg).expect("Error serializinng get users msg");
    let result = stream
        .write(&bytes)
        .await
        .map_err(|e| println!("Error writing bytes {}", e));
    assert!(result.is_ok(), "Error writing to stream");
    let mut buf = vec![0; 1024];
    let result = stream.read(&mut buf).await;
    assert!(result.is_ok(), "Error reading resp bytes");
    let response_msg: IPCResponse =
        serde_json::from_slice(&buf).expect("Error deserialzing ipc response");
    if let IPCResponse::SendUsers(users) = response_msg {
        for user in users.users {
            println!("--TEST {}", user.display_name);
        }
    }
}

#[tokio::test]
async fn test_ipc_basic() {
    //Test setup/cleanup
    let test_paths = vec!["./test_path"];
    const NUM_MSGS: usize = 5;

    logger::init_tracing();
    let cleanup = Cleanup {
        test_paths: &test_paths,
    };

    cleanup.remove_test_paths();
    //Send RunMessage
    let (runmessage_tx, mut runmessage_rx) = mpsc::channel(100);
    //Used to receive and send data back out through the socket
    let (_data_tx, data_rx) = mpsc::channel(100);

    //Spawn ipc handler
    let runtime_tx = runmessage_tx.clone();
    tokio::spawn(async move { ipc::listen(data_rx, runtime_tx, "7878".to_string()).await });

    let client = Client::new(test_paths[0]).await;
    let node_id = client.get_node_id();

    let result = match timeout(Duration::from_secs(5), async {
        loop {
            let result = TcpStream::connect("127.0.0.1:7878").await;
            if result.is_ok() {
                println!("Connected on 127.0.0.1:7878");
                return result;
            }
            sleep(Duration::from_secs(1)).await;
        }
    })
    .await
    {
        Ok(stream) => stream,
        Err(e) => panic!("Failed to connect to tcp stream {}", e),
    };
    let mut stream = result.unwrap();

    let test_message = SendMessageMsg {
        display_name: "test".to_string(),
        content: "Test".to_string(),
    };

    let test_message = IPCMessage::SendMessage(test_message);
    let test_json = serde_json::to_string(&test_message).expect("Error converting to string");
    println!("{}", test_json);
    let bytes = serde_json::to_vec(&test_message).expect("failed to serialize message");

    let recv_check = tokio::spawn(async move {
        let mut num_recv = 0;
        loop {
            let result = match timeout(Duration::from_secs(5), async {
                if let Some(_) = runmessage_rx.recv().await {
                    num_recv += 1;
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Timeout error"))
                }
            })
            .await
            {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            };

            assert!(result.is_ok(), "Failed to assert recv runtime message");
            if num_recv == 5 {
                break;
            }
        }
    });

    for i in 0..NUM_MSGS {
        let result = stream
            .write(&bytes)
            .await
            .map_err(|e| println!("error writing to streamm: {e}"));
        //assert!(result.is_ok(), "Failed to write to stream");
        println!("Succesfully wrote {} bytes", result.unwrap());
        println!("Succesfully sent message: {}", i);

        let _ = sleep(Duration::from_secs(1)).await;
    }
    recv_check.await.expect("Error closing recv check handle");
}
