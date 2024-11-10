use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info};

use std::collections::VecDeque;

use anyhow::Result;

use crate::utils::enums::{RunMessage, UserStatus};
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
    info!("Listening on localhost:7878...");
    match listener.accept().await {
        Ok((mut socket, _)) => {
            info!("Recieved data on 7878");
            let mut buf = vec![0; 1024];

            //let mut reader = BufReader::new(&mut socket);
            /*
                            match socket.read_exact(&mut buf).await {
                                Ok(num_bytes) => info!("Succesfully received {} bytes", num_bytes),
                                Err(e) => {
                                    error!("Failed to read from socket {}", e);
                                    continue;
                                }
                            }
            */

            loop {
                let num_bytes = socket.read(&mut buf).await.expect("Error reading...");
                buf = buf[0..num_bytes].to_vec();

                let ipc_message = match serde_json::from_slice::<IPCMessage>(&buf) {
                    Ok(ipc_message) => ipc_message,
                    Err(e) => {
                        error!("Error deserializing IPC message: {e}");
                        continue;
                    }
                };

                let run_message = match ipc_message {
                    IPCMessage::Adduser(add_user) => {
                        RunMessage::Adduser(add_user.node_id, add_user.display_name)
                    }
                    IPCMessage::UpdateStatus(update_status) => {
                        RunMessage::UpdateStatus(update_status.node_id, update_status.user_status)
                    }
                    IPCMessage::SendMessage(send_message) => {
                        info!("------------- IPC message received");
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
                info!("Forwarded IPC message to runtime...");
            }
        }
        Err(e) => {
            error!("Error opening socket to 7878 {}", e);
            Err(anyhow::anyhow!("Timeout error"))
        }
    }
    /*if let Some(response) = rx.recv().await {
        let bytes = serialize_ipc_message(response)?;
        socket.write_all(&bytes).await?;
    }*/
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
