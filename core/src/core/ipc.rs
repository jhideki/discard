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
    Shutdown,
    GetNodeId,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IPCResponse {
    SendUsers(SendUsersResp),
    SendUser(User),
    Error(IPCErrorType),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct IPCErrorType {
    #[serde(rename = "errorMessage")]
    pub error: String,
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

                let run_message = match serde_json::from_slice::<RunMessage>(&buf) {
                    Ok(ipc_message) => ipc_message,
                    Err(e) => {
                        error!("Error deserializing IPC message: {e}");
                        continue;
                    }
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
