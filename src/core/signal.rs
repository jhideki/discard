use anyhow::Result;
use iroh::client::Iroh;
use iroh::net::endpoint::get_remote_node_id;
use iroh::node::ProtocolHandler;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

use crate::core::rtc;
use crate::utils::enums::SessionType;

#[derive(Debug)]
struct ICEExchange {
    client: Iroh,
    max_size: usize,
    candidate: RTCIceCandidate,
}
//TODO impl ProtocolHandler for ICEExchange

#[derive(Debug)]
pub struct SDPExchange {
    client: Iroh,
    max_size: usize,
    local_sessions: Mutex<Vec<RTCSessionDescription>>,
    pub remote_sessions: Mutex<Vec<RTCSessionDescription>>,
    pub sdp_notify: Notify,
}

impl ProtocolHandler for SDPExchange {
    fn accept(self: Arc<Self>, conn: iroh::net::endpoint::Connecting) -> BoxedFuture<Result<()>> {
        Box::pin(async move {
            let connection = conn.await?;
            let node_id = get_remote_node_id(&connection);
            let (mut send, mut recv) = connection.accept_bi().await?;
            let remote_session = recv.read_to_end(self.max_size).await?;
            self.sdp_notify.notify_one();
            //TODO: set remote sessions
            //
            let mut sessions = self.local_sessions.lock().await;
            for s in sessions.drain(..) {
                let bytes = serde_json::to_vec(&s)?;
                send.write_all(&bytes).await?;
            }

            send.finish().await?;
            Ok(())
        })
    }
}

impl SDPExchange {
    pub fn new(client: Iroh) -> Arc<Self> {
        let max_size = std::mem::size_of::<RTCSessionDescription>();
        Arc::new(Self {
            client,
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
}
