mod utils;

use discard::core::client::Client;
use discard::core::ipc::{self, IPCMessage, SendMessageMsg};
use discard::utils::logger;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use utils::Cleanup;

#[tokio::test]
async fn test_ipc() {
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
    tokio::spawn(async move { ipc::listen(data_rx, runtime_tx).await });

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
        node_id,
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
