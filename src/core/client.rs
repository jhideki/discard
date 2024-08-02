use crate::core::rtc::{APIWrapper, Connection, RTCConfigurationWrapper};
use crate::core::signal::{IdExchange, Session, SessionExchange};
use crate::utils::constants::ID_APLN;
use crate::utils::{
    constants::{SDP_ALPN, STUN_SERVERS},
    enums::{ConnType, SessionType},
    types::NodeId,
};

use anyhow::Result;
use iroh::{
    blobs::store::fs::Store,
    node::{Builder, Node},
};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, instrument, Span};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::configuration::RTCConfiguration,
};

use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug)]
struct RTCConfig {
    api: APIWrapper,
    config: RTCConfigurationWrapper,
}

#[derive(Debug)]
pub struct Client {
    pub connections: Vec<Connection>,
    rtc_config: RTCConfig,
    node: Node<Store>,
    session_exchange: Arc<SessionExchange>,
    id_exchange: Arc<IdExchange>,
}

impl Client {
    pub async fn new(root: &str) -> Self {
        let builder = Builder::default()
            .persist(root)
            .await
            .expect("Failed to create store")
            .disable_docs()
            .build()
            .await
            .expect("Failed to build node");

        let session_exchange = SessionExchange::new(builder.endpoint().clone());
        let id_exchange = IdExchange::new(builder.endpoint().clone());
        let node = builder
            .accept(SDP_ALPN, session_exchange.clone())
            .accept(ID_APLN, id_exchange.clone())
            .spawn()
            .await
            .expect("Failed to spawn node");

        let stun_servers = STUN_SERVERS.iter().map(|&s| s.to_string()).collect();

        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: stun_servers,
                ..Default::default()
            }],
            ..Default::default()
        };

        let mut m = MediaEngine::default();
        let _ = m.register_default_codecs();

        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut m)
            .expect("Failed to regiestire interceptors");
        let api = APIBuilder::new()
            .with_media_engine(m)
            .with_interceptor_registry(registry)
            .build();
        Client {
            connections: Vec::new(),
            rtc_config: RTCConfig {
                api: APIWrapper(api),
                config: RTCConfigurationWrapper(config),
            },
            node,
            session_exchange,
            id_exchange,
        }
    }

    #[instrument(skip(self),fields(node_id = self.node.node_id().to_string()))]
    pub async fn init_connection(&mut self, remote_node_id: NodeId) -> Result<()> {
        let mut conn = Connection::new(
            &self.rtc_config.api,
            self.rtc_config.config.clone(),
            ConnType::Offerer,
            self.session_exchange.clone(),
            remote_node_id,
            self.connections.len(),
        )
        .await;

        //Typical WebRTC steps...
        conn.create_data_channel().await;
        info!("Created data channel!");
        conn.init_ice_handler().await;
        info!("Listening for ice candidates");
        conn.offer().await?;
        info!("Created offer!");
        conn.init_remote_handler().await?;

        self.send_remote_node_id(remote_node_id).await?;

        info!("Listening for remote traffic");
        conn.monitor_connection().await;

        //Returns handles to worker threads
        let handles = conn.get_task_handles();
        &self.connections.push(conn);
        Ok(())
    }

    #[instrument(skip(self),fields(node_id = self.node.node_id().to_string()))]
    pub async fn send_remote_node_id(&self, remote_node_id: NodeId) -> Result<()> {
        self.id_exchange.send_node_id(remote_node_id).await?;
        Ok(())
    }

    #[instrument(skip(self),fields(node_id = self.node.node_id().to_string()))]
    pub async fn get_remote_node_id(&self) -> Result<NodeId> {
        let (tx, mut rx) = mpsc::channel(10);
        let id_exchange = self.id_exchange.clone();
        id_exchange.init(tx.clone()).await;

        if let Some(remote_node_id) = rx.recv().await {
            return Ok(remote_node_id);
        }
        Err(anyhow::anyhow!("Error retreiving remote node id"))
    }

    pub async fn receive_connection(&self) -> Result<()> {
        if let Ok(remote_node_id) = self.get_remote_node_id().await {
            let mut conn = Connection::new(
                &self.rtc_config.api,
                self.rtc_config.config.clone(),
                ConnType::Offerer,
                self.session_exchange.clone(),
                remote_node_id,
                self.connections.len(),
            )
            .await;
            conn.register_data_channel().await;
            conn.init_ice_handler().await;
            conn.init_remote_handler().await?;
            conn.answer().await;
        }

        Ok(())
    }

    pub fn get_node_id(&self) -> NodeId {
        self.node.node_id()
    }

    pub async fn send_message(&self, conn_id: usize, message: String) -> Result<()> {
        let conn = &self.connections[conn_id];
        conn.send_dc_message(message).await?;
        //TODO: write message to db

        Ok(())
    }

    pub async fn get_message(&self, conn_id: usize) -> Result<String> {
        let conn = &self.connections[conn_id];
        if let Some(message) = conn.get_message().await {
            return Ok(message);
        }
        Err(anyhow::anyhow!("No messages in queue!"))
    }
}
