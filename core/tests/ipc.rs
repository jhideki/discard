mod utils;

use discard::core::client::{run, Client};
use discard::core::ipc::{self, IPCMessage, IPCResponse, SendMessageMsg, SendUsersResp};
use discard::utils::logger;
use iroh::net::key::PublicKey;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use utils::Cleanup;

use std::collections::HashSet;

#[tokio::test]
async fn test_ipc_get_messages() {
    //Test setup/cleanup
    let test_paths = vec!["./test_ipc_get_messages"];

    logger::init_tracing();
    let cleanup = Cleanup {
        test_paths: &test_paths,
    };

    cleanup.remove_test_paths();
    //Send RunMessage
    let (runmessage_tx, runmessage_rx) = mpsc::channel(100);
    //Used to receive and send data back out through the socket
    let (data_tx, data_rx) = mpsc::channel(100);

    //Spawn ipc handler
    let runtime_tx = runmessage_tx.clone();
    tokio::spawn(async move { ipc::listen(data_rx, runtime_tx, "7878".to_string()).await });

    let mut client = Client::new(test_paths[0]).await;
    let key = PublicKey::from_bytes(&[0; 32]).expect("Failed to gen dummy key");
    let mut test_users = HashSet::new();
    test_users.insert("test_user1".to_string());
    test_users.insert("test_user2".to_string());
    client
        .add_user(key, "test_user1".to_string())
        .expect("Failed to add dummy user");

    client
        .add_user(key, "test_user2".to_string())
        .expect("Failed to add dummy user");

    tokio::spawn(async move {
        run(client, runmessage_tx, runmessage_rx, data_tx)
            .await
            .expect("Failed to run client");
    });

    let result = match timeout(Duration::from_secs(5), async {
        loop {
            let result = TcpStream::connect("127.0.0.1:7878").await;
            if result.is_ok() {
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
    let num_bytes = result.unwrap();

    buf = buf[0..num_bytes].to_vec();
    let response_msg: IPCResponse =
        serde_json::from_slice(&buf).expect("Error deserialzing ipc response");
    let IPCResponse::SendUsers(users) = response_msg;

    for user in users.users {
        test_users.remove(&user.display_name);
    }

    assert!(
        test_users.len() == 0,
        "Test users do match values retreived from the client"
    );
}

#[tokio::test]
async fn test_ipc_basic() {
    //Test setup/cleanup
    let test_paths = vec!["./test_ipc_basic"];
    const NUM_MSGS: usize = 5;

    logger::init_tracing();
    let cleanup = Cleanup {
        test_paths: &test_paths,
    };

    cleanup.remove_test_paths();
    //Send RunMessage
    let (runmessage_tx, mut runmessage_rx) = mpsc::channel(100);
    //Used to receive and send data back out through the socket
    let (ipc_tx, ipc_rx) = mpsc::channel(100);

    //Spawn ipc handler
    let runtime_tx = runmessage_tx.clone();
    tokio::spawn(async move { ipc::listen(ipc_rx, runtime_tx, "7879".to_string()).await });

    let result = match timeout(Duration::from_secs(5), async {
        loop {
            let result = TcpStream::connect("127.0.0.1:7879").await;
            if result.is_ok() {
                println!("Connected on 127.0.0.1:7879");
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
    let bytes = serde_json::to_vec(&test_message).expect("failed to serialize message");

    //Simulate runtime
    let recv_check = tokio::spawn(async move {
        let mut num_recv = 0;
        let result = match timeout(Duration::from_secs(60), async {
            while let Some(msg) = runmessage_rx.recv().await {
                num_recv += 1;
                if num_recv == 5 {
                    break;
                }
                let runtime_response = ipc_tx
                    .send(IPCResponse::SendUsers(SendUsersResp { users: Vec::new() }))
                    .await;
                assert!(
                    runtime_response.is_ok(),
                    "Error: {:?}",
                    runtime_response.err()
                )
            }
        })
        .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        };

        assert!(
            result.is_ok(),
            "Failed to assert recv runtime message. Err: {:?}",
            result.unwrap_err()
        );
    });

    println!("num messages: {:?}", NUM_MSGS);

    for i in 0..NUM_MSGS {
        let result = stream
            .write(&bytes)
            .await
            .map_err(|e| println!("error writing to streamm: {e}"));
        assert!(result.is_ok(), "Failed to write to stream");
        println!("Succesfully wrote {} bytes", result.unwrap());
        println!("Succesfully sent message: {}", i);

        let _ = sleep(Duration::from_secs(1)).await;
    }
    recv_check.await.expect("Error closing recv check handle");
}
