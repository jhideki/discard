use anyhow::Result;
use iroh::net::endpoint::get_remote_node_id;
use iroh::net::Endpoint;
use iroh::node::ProtocolHandler;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{debug, error, info};
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::utils::enums::{MessageType, RunMessage, SignalMessage};
use crate::utils::types::NodeId;
use crate::utils::{
    constants::{SDP_ALPN, SIGNAL_ALPN},
    types::BoxedFuture,
};

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
    session_tx: Mutex<Option<mpsc::Sender<Session>>>,
    node_id_tx: Mutex<Option<mpsc::Sender<NodeId>>>,
    has_remote_id: Mutex<bool>,
}

impl ProtocolHandler for SessionExchange {
    fn accept(self: Arc<Self>, conn: iroh::net::endpoint::Connecting) -> BoxedFuture<Result<()>> {
        Box::pin(async move {
            //Open a connection to peer
            let connection = conn.await?;

            let (mut _send, mut recv) = connection.accept_bi().await?;

            //Set remote node id
            let mut has_remote_id = self.has_remote_id.lock().await;
            if !*has_remote_id {
                let remote_node_id = get_remote_node_id(&connection)?;
                let tx = self.node_id_tx.lock().await;

                if let Some(tx) = tx.as_ref() {
                    tx.send(remote_node_id).await?;
                }
                *has_remote_id = true;
            }

            //Read session info
            match recv.read_to_end(2000).await {
                Ok(buf) => {
                    info!("Receieved data!");
                    match bincode::deserialize(&buf) {
                        Ok(remote_session) => {
                            //Notify that sdp/ice candidate has been found to signal client struct
                            let tx = self.session_tx.lock().await;
                            if let Some(tx) = tx.as_ref() {
                                tx.send(remote_session).await?;
                            }
                        }
                        Err(e) => error!("Error deserializing session: {}", e),
                    }
                }
                Err(e) => error!("Error reading buffer: {}", e),
            }

            Ok(())
        })
    }
}

impl SessionExchange {
    pub fn new(endpoint: Endpoint) -> Arc<Self> {
        Arc::new(Self {
            endpoint,
            session_tx: Mutex::new(None),
            node_id_tx: Mutex::new(None),
            has_remote_id: Mutex::new(false),
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

#[derive(Debug)]
pub struct Signaler {
    endpoint: Endpoint,
    sender: Mutex<Option<mpsc::Sender<RunMessage>>>,
}
impl ProtocolHandler for Signaler {
    fn accept(self: Arc<Self>, conn: iroh::net::endpoint::Connecting) -> BoxedFuture<Result<()>> {
        Box::pin(async move {
            let connection = conn.await?;
            let (_send, mut recv) = connection.accept_bi().await?;
            let buf = recv.read_to_end(64).await?;

            let status = bincode::deserialize::<SignalMessage>(&buf)?;

            let sender = self.sender.lock().await;

            //Notify listner to set peer status to 'online'
            if let Some(sender) = sender.as_ref() {
                match status {
                    SignalMessage::SendConnection => {
                        sender.send(RunMessage::ReceiveMessage).await;
                    }
                    SignalMessage::Online => {
                        sender.send(RunMessage::Online).await;
                    }
                }
            }

            Ok(())
        })
    }
}

impl Signaler {
    pub fn new(endpoint: Endpoint) -> Arc<Self> {
        Arc::new(Self {
            endpoint,
            sender: Mutex::new(None),
        })
    }
    pub async fn notify_online(&self, remote_node_id: NodeId) -> Result<()> {
        let conn = &self
            .endpoint
            .connect_by_node_id(remote_node_id, SIGNAL_ALPN)
            .await?;
        let (mut send, _recv) = conn.open_bi().await?;
        let buf = bincode::serialize(&SignalMessage::Online)?;
        send.write_all(&buf).await;
        send.finish().await;
        Ok(())
    }

    pub async fn notify_connection(&self, remote_node_id: NodeId) -> Result<()> {
        let conn = &self
            .endpoint
            .connect_by_node_id(remote_node_id, SIGNAL_ALPN)
            .await?;
        let (mut send, _recv) = conn.open_bi().await?;
        let buf = bincode::serialize(&SignalMessage::SendConnection)?;
        send.write_all(&buf).await;
        send.finish().await;
        Ok(())
    }

    pub async fn init_sender(&self, sender: mpsc::Sender<RunMessage>) {
        let mut online_sender = self.sender.lock().await;
        *online_sender = Some(sender);
    }
}
