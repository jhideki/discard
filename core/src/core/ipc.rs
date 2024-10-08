use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::info;

use std::collections::VecDeque;

use anyhow::Result;

use crate::utils::enums::RunMessage;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct IPCMessage {
    pub run_message: RunMessage,
    pub content: Vec<u8>,
}

pub async fn listen(
    mut rx: mpsc::Receiver<IPCMessage>,
    runtime_tx: mpsc::Sender<RunMessage>,
) -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878").await?;
    loop {
        let (mut socket, _) = listener.accept().await?;
        info!("Listening on localhost:7878...");
        {
            let mut length_buf = [0u8; 4];

            let mut reader = BufReader::new(&mut socket);

            let _ = reader.read_exact(&mut length_buf);

            let length = u32::from_be_bytes(length_buf);
            let mut body_buf = vec![0u8; length as usize];
            let _ = reader.read_exact(&mut body_buf).await;
            let ipc_message = serde_json::from_slice::<IPCMessage>(&body_buf)
                .expect("failed to Deserialize ipc message");
            println!("{}", String::from_utf8(ipc_message.content).unwrap());
            runtime_tx
                .send(ipc_message.run_message)
                .await
                .expect("Failed to send run message from listener");
        }

        if let Some(response) = rx.recv().await {
            let bytes = serialize_ipc_message(response)?;
            socket.write_all(&bytes).await?;
        }
    }
}

//custom serializetion for ipc. will probalby change later
//TODO: remove vecdeque impl
pub fn serialize_ipc_message(ipc_message: IPCMessage) -> Result<Vec<u8>> {
    let message_buf = bincode::serialize(&ipc_message)?;
    let mut message_buf = VecDeque::from(message_buf);
    //first 4 bytes of message
    let len = message_buf.len().to_ne_bytes();
    let _ = len.into_iter().rev().map(|b| message_buf.push_front(b));

    let buf = Vec::from(message_buf);
    Ok(buf)
}
