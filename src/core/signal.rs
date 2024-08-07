use anyhow::Result;
use iroh::net::endpoint::get_remote_node_id;
use iroh::net::Endpoint;
use iroh::node::ProtocolHandler;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
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
//TODO: maybe change mpsc to a oneshot channel? May need to rework this design.
#[derive(Debug)]
pub struct SessionExchange {
    endpoint: Endpoint,
    max_size: usize,
    session_tx: Mutex<Option<mpsc::Sender<Session>>>,
    node_id_tx: Mutex<Option<mpsc::Sender<NodeId>>>,
}

impl ProtocolHandler for SessionExchange {
    fn accept(self: Arc<Self>, conn: iroh::net::endpoint::Connecting) -> BoxedFuture<Result<()>> {
        Box::pin(async move {
            //Open a connection to peer
            let connection = conn.await?;

            let (mut _send, mut recv) = connection.accept_bi().await?;

            //Set remote node id
            let remote_node_id = get_remote_node_id(&connection)?;
            let tx = self.node_id_tx.lock().await;
            if let Some(tx) = tx.as_ref() {
                tx.send(remote_node_id).await?;
            }

            //Read session info
            let bytes = recv.read_to_end(self.max_size).await?;
            let remote_session = bincode::deserialize(&bytes)?;

            //Notify that sdp/ice candidate has been found to signal client struct
            let tx = self.session_tx.lock().await;
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
            session_tx: Mutex::new(None),
            node_id_tx: Mutex::new(None),
        })
    }

    //Used to set the channel to Some() value
    pub async fn init_session_sender(&self, session_sender: mpsc::Sender<Session>) {
        let mut tx = self.session_tx.lock().await;
        *tx = Some(session_sender);
    }

    //Used to set the channel to Some() value
    pub async fn init_id_sender(&self, id_sender: mpsc::Sender<NodeId>) {
        let mut tx = self.node_id_tx.lock().await;
        *tx = Some(id_sender);
    }

    pub async fn send_session(&self, node_id: NodeId, session: Session) -> Result<()> {
        let conn = &self.endpoint.connect_by_node_id(node_id, SDP_ALPN).await?;
        let (mut send, _recv) = conn.open_bi().await?;
        let bytes = bincode::serialize(&session)?;
        send.write_all(&bytes).await?;
        send.finish().await?;
        Ok(())
    }
}
