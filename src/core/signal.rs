use anyhow::Result;
use iroh::net::Endpoint;
use iroh::node::ProtocolHandler;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify};
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::utils::constants::ID_APLN;
use crate::utils::types::NodeId;
use crate::utils::{constants::SDP_ALPN, types::BoxedFuture};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub ice_candidate: Option<RTCIceCandidate>,
    pub sdp: Option<RTCSessionDescription>,
}

#[derive(Debug)]
pub struct SessionExchange {
    endpoint: Endpoint,
    max_size: usize,
    local_sessions: Mutex<Vec<RTCSessionDescription>>,
    pub remote_sessions: Mutex<Vec<RTCSessionDescription>>,
    pub sdp_notify: Notify,
}

impl ProtocolHandler for SessionExchange {
    fn accept(self: Arc<Self>, conn: iroh::net::endpoint::Connecting) -> BoxedFuture<Result<()>> {
        Box::pin(async move {
            let connection = conn.await?;
            let (mut send, mut recv) = connection.accept_bi().await?;
            let bytes = recv.read_to_end(self.max_size).await?;
            let remote_session = bincode::deserialize(&bytes)?;
            self.sdp_notify.notify_one();
            let mut sessions = self.local_sessions.lock().await;
            sessions.push(remote_session);
            for s in sessions.drain(..) {
                let bytes = bincode::serialize(&s)?;
                send.write_all(&bytes).await?;
            }

            send.finish().await?;
            Ok(())
        })
    }
}

impl SessionExchange {
    pub fn new(endpoint: Endpoint) -> Arc<Self> {
        let max_size = std::mem::size_of::<RTCSessionDescription>();
        Arc::new(Self {
            endpoint,
            max_size,
            local_sessions: Mutex::new(Vec::new()),
            remote_sessions: Mutex::new(Vec::new()),
            sdp_notify: Notify::new(),
        })
    }
    async fn add_local(&mut self, session: RTCSessionDescription) {
        let mut sessions = self.local_sessions.lock().await;
        sessions.push(session);
    }
    async fn add_remote(&mut self, session: RTCSessionDescription) {
        let mut sessions = self.remote_sessions.lock().await;
        sessions.push(session);
    }
    pub async fn send_session(&self, node_id: NodeId, session: Session) -> Result<()> {
        let conn = &self.endpoint.connect_by_node_id(node_id, SDP_ALPN).await?;
        let mut send = conn.open_uni().await?;
        let bytes = bincode::serialize(&session)?;
        send.write_all(&bytes).await?;
        send.finish().await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct IdExchange {
    endpoint: Endpoint,
    tx: Mutex<Option<mpsc::Sender<NodeId>>>,
    is_initilized: bool,
}
impl ProtocolHandler for IdExchange {
    fn accept(self: Arc<Self>, conn: iroh::net::endpoint::Connecting) -> BoxedFuture<Result<()>> {
        Box::pin(async move {
            let connection = conn.await?;
            let mut recv = connection.accept_uni().await?;
            let mut buf: Vec<u8> = Vec::new();
            if let Some(_) = recv.read(&mut buf).await? {
                if let Ok(node_id) = bincode::deserialize::<NodeId>(&buf) {
                    if self.is_initilized {
                        let tx = self.tx.lock().await.unwrap();
                    }
                }
            }
            Ok(())
        })
    }
}
impl IdExchange {
    pub fn new(endpoint: Endpoint) -> Arc<Self> {
        Arc::new(Self {
            endpoint,
            tx: Mutex::new(None),
            is_initilized: false,
        })
    }
    pub async fn init(&mut self, sender: mpsc::Sender<NodeId>) {
        let mut tx = self.tx.lock().await;
        *tx = Some(sender);
        self.is_initilized = true;
    }
    pub async fn send_node_id(&self, node_id: NodeId) -> Result<()> {
        let conn = &self.endpoint.connect_by_node_id(node_id, ID_APLN).await?;
        let mut send = conn.open_uni().await?;
        let bytes = bincode::serialize(&self.endpoint.node_id())?;
        send.write_all(&bytes);
        send.finish().await?;
        Ok(())
    }
}
