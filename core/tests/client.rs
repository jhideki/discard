mod utils;

use discard::core::ipc::IPCResponse;
use discard::utils::types::TextMessage;
use tokio::sync::mpsc;

use discard::core::client::{self, Client};
use discard::utils::enums::RunMessage;
use discard::utils::logger;
use utils::Cleanup;

//Create two clients and test sdp exchange via sending store bytes from iroh
#[tokio::test]
async fn test_data_channel() {
    logger::init_tracing();
    let test_paths = vec![
        "./test_data_channel1".to_string(),
        "./test_data_channel2".to_string(),
        "./test_data_channel3".to_string(),
    ];

    //Will remove test paths again at the end of the test
    let cleanup = Cleanup {
        test_paths: test_paths,
    };
    cleanup.remove_test_paths();

    let p1 = Client::new(test_paths[0]).await;
    let p2 = Client::new(test_paths[1]).await;

    //peer 1 channel to simulate client receiving a message
    let (tx1, rx1) = mpsc::channel::<RunMessage>(10);
    let (ipc_tx, _) = mpsc::channel::<IPCResponse>(10);
    println!("---------spawning peer 1");
    let sender = tx1.clone();
    tokio::spawn(async move {
        let result = client::run(p1, sender, rx1, ipc_tx).await;
        assert!(result.is_ok());
    });

    let result = tx1
        .send(RunMessage::RecvConn(
            discard::utils::enums::SessionType::Chat,
        ))
        .await;

    assert!(result.is_ok());
    println!("recievemessage sent");
    assert!(result.is_ok());

    //peer 2 channel to send peer 1 a message
    let (tx2, rx2) = mpsc::channel(10);

    //Used to transmit ipc message back to sender
    let (ipc_tx2, _) = mpsc::channel::<IPCResponse>(10);
    println!("---------spawning peer 2");
    let sender = tx2.clone();
    tokio::spawn(async move {
        println!("----------recieved node id from peer");
        //peer 2 channel to simulate client running on their machine
        let result = client::run(p2, sender.clone(), rx2, ipc_tx2).await;
        assert!(result.is_ok());
    });

    let text_message = TextMessage {
        content: "test".to_string(),
        timestamp: chrono::Utc::now(),
    };
    let result = tx2
        .send(RunMessage::SendMessage(
            "TEMP".to_string(),
            "test".to_string(),
        ))
        .await;
    assert!(result.is_ok());
    assert!(result.is_ok());
    println!("--------- p1 and p2 done");
}
