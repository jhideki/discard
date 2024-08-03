use anyhow::Result;
use iroh::net::Endpoint;
use iroh::node::ProtocolHandler;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info};
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

//Used to send SDP and ICE candidates to peer
#[derive(Debug)]
pub struct SessionExchange {
    endpoint: Endpoint,
    max_size: usize,
    tx: Mutex<Option<mpsc::Sender<Session>>>,
}

impl ProtocolHandler for SessionExchange {
    fn accept(self: Arc<Self>, conn: iroh::net::endpoint::Connecting) -> BoxedFuture<Result<()>> {
        Box::pin(async move {
            debug!("Recieved data from peer");
            //Open a connection to peer
            let connection = conn.await?;
            let mut recv = connection.accept_uni().await?;
            let bytes = recv.read_to_end(self.max_size).await?;
            let remote_session = bincode::deserialize(&bytes)?;

            //Notify that sdp/ice candidate has been found to signal client struct
            let tx = self.tx.lock().await;
            if let Some(tx) = tx.as_ref() {
                tx.send(remote_session).await?;
            }

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
            tx: Mutex::new(None),
        })
    }

    //Used to set the channel to Some() value
    pub async fn init(&self, sender: mpsc::Sender<Session>) {
        let mut tx = self.tx.lock().await;
        *tx = Some(sender);
    }

    pub async fn send_session(&self, node_id: NodeId, session: Session) -> Result<()> {
        let conn = &self.endpoint.connect_by_node_id(node_id, SDP_ALPN).await?;
        let mut send = conn.open_uni().await?;
        debug!("Opened channel");
        let bytes = bincode::serialize(&session)?;
        send.write_all(&bytes).await?;
        send.finish().await?;
        debug!("Sent session");
        Ok(())
    }
}

//Used to send node_id to peer
#[derive(Debug)]
pub struct IdExchange {
    endpoint: Endpoint,
    tx: Mutex<Option<mpsc::Sender<NodeId>>>,
}
impl ProtocolHandler for IdExchange {
    fn accept(self: Arc<Self>, conn: iroh::net::endpoint::Connecting) -> BoxedFuture<Result<()>> {
        debug!("Inside accept");
        Box::pin(async move {
            let connection = conn.await?;
            debug!("Connection received!");
            let mut recv = connection.accept_uni().await?;
            let mut buf: Vec<u8> = Vec::new();
            if let Some(_) = recv.read(&mut buf).await? {
                if let Ok(node_id) = bincode::deserialize::<NodeId>(&buf) {
                    let tx = self.tx.lock().await;
                    if let Some(tx) = tx.as_ref() {
                        tx.send(node_id).await?;
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
        })
    }
    pub async fn init(&self, sender: mpsc::Sender<NodeId>) {
        let mut tx = self.tx.lock().await;
        *tx = Some(sender);
        info!("channel initialized");
    }
    pub async fn send_node_id(&self, node_id: NodeId) -> Result<()> {
        debug!("Inside send_node_id in signal.rs");
        let self_node_id = &self.endpoint.node_id().fmt_short();
        debug!("Trying to open connection... {}", self_node_id);
        let conn = &self.endpoint.connect_by_node_id(node_id, ID_APLN).await?;
        let mut send = conn.open_uni().await?;
        debug!("Connection Opened");
        let bytes = bincode::serialize(&self.endpoint.node_id())?;
        send.write_all(&bytes).await?;
        info!("Data sent!");
        send.finish().await?;
        Ok(())
    }
}
