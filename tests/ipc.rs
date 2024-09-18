use discard::core::client::{self, Client};
use discard::core::ipc::{self, IPCMessage};
use discard::utils::enums::RunMessage;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[tokio::test]
async fn test_ipc() {
    //Send RunMessage
    let (runmessage_tx, runmessage_rx) = mpsc::channel(100);
    //Used to receive and send data back out through the socket
    let (data_tx, data_rx) = mpsc::channel(100);

    let runtime_tx = runmessage_tx.clone();
    tokio::spawn(async move { ipc::listen(data_rx, runtime_tx).await });

    let client = Client::new("./").await;
    let client_data_tx = data_tx.clone();
    let client_tx = runmessage_tx.clone();
    tokio::spawn(async move { client::run(client, client_tx, runmessage_rx, client_data_tx) });

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

    let content = "Test".to_string();
    let bytes = bincode::serialize(&content).expect("");

    let test_message = IPCMessage {
        run_message: RunMessage::ReceiveMessage,
        content: bytes,
    };

    let bytes = bincode::serialize(&test_message).expect("failed to serialize message");

    let result = stream.write_all(&bytes).await;
    assert!(result.is_ok(), "Failed to write to stream");
}
