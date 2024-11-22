use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{error, info};

use anyhow::Result;

use crate::database::models::User;
use crate::utils::enums::{RunMessage, UserStatus};
use crate::utils::types::{NodeId, TextMessage};

//Structs are public for UTs
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IPCMessage {
    AddUser(AddUserMsg),
    UpdateStatus(UpdateStatusMsg),
    SendMessage(SendMessageMsg),
    GetUsers,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IPCResponse {
    SendUsers(SendUsersResp),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SendUsersResp {
    #[serde(rename = "users")]
    pub users: Vec<User>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct AddUserMsg {
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
    pub display_name: String,
    #[serde(rename = "content")]
    pub content: String,
}

pub async fn listen(
    mut rx: mpsc::Receiver<IPCResponse>,
    runtime_tx: mpsc::Sender<RunMessage>,
    port: String,
) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(addr).await?;
    info!("IPC listener running on localhost:{}...", port);
    match listener.accept().await {
        Ok((mut socket, _)) => {
            let mut buf = vec![0; 1024];
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
                    IPCMessage::AddUser(add_user) => {
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
                        RunMessage::SendMessage(send_message.display_name, msg_content)
                    }
                    IPCMessage::GetUsers => RunMessage::GetUsers,
                };

                runtime_tx
                    .send(run_message)
                    .await
                    .expect("Failed to send run message from listener");
                info!("Forwarded IPC message to runtime...");

                if let Some(response) = rx.recv().await {
                    let bytes = serde_json::to_vec(&response)?;
                    info!("Num bytes in core/ipc: {}", bytes.len());
                    socket.write(&bytes).await?;
                }
            }
        }
        Err(e) => {
            error!("Error opening socket to 7878 {}", e);
            Err(anyhow::anyhow!("Timeout error"))
        }
    }
}
