use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::info;

use std::collections::VecDeque;

use anyhow::Result;

use crate::utils::enums::{RunMessage, RunMessageType, UserStatus};
use crate::utils::types::{NodeId, TextMessage};

//Structs are public for UTs
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IPCMessage {
    Adduser(AdduserMsg),
    UpdateStatus(UpdateStatusMsg),
    SendMessage(SendMessageMsg),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct AdduserMsg {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct UpdateStatusMsg {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "userStatus")]
    pub user_status: UserStatus,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SendMessageMsg {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "content")]
    pub content: String,
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
            let ipc_message: IPCMessage = serde_json::from_slice::<IPCMessage>(&body_buf)
                .expect("failed to Deserialize ipc message");

            let run_message = match ipc_message {
                IPCMessage::Adduser(add_user) => {
                    RunMessage::Adduser(add_user.node_id, add_user.display_name)
                }
                IPCMessage::UpdateStatus(update_status) => {
                    RunMessage::UpdateStatus(update_status.node_id, update_status.user_status)
                }
                IPCMessage::SendMessage(send_message) => {
                    let msg_content = TextMessage {
                        content: send_message.content,
                        timestamp: chrono::Utc::now(),
                    };
                    RunMessage::SendMessage(send_message.node_id, msg_content)
                }
            };

            runtime_tx
                .send(run_message)
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
