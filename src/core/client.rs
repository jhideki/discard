use crate::core::rtc::{APIWrapper, Connection, RTCConfigurationWrapper};
use crate::core::signal::SessionExchange;
use crate::utils::{
    constants::{SDP_ALPN, STUN_SERVERS},
    enums::ConnType,
    types::NodeId,
};

use anyhow::Result;
use iroh::{
    blobs::store::fs::Store,
    node::{Builder, Node},
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, Span};
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
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

//Used to manage connection states
#[derive(Debug)]
struct ConnSubscriber {
    dc_rx: mpsc::Receiver<String>,
    conn_rx: mpsc::Receiver<RTCPeerConnectionState>,
    conn: Connection,
}

#[derive(Debug)]
pub struct Client {
    pub connections: Vec<ConnSubscriber>,
    rtc_config: RTCConfig,
    node: Node<Store>,
    session_exchange: Arc<SessionExchange>,
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
        let node = builder
            .accept(SDP_ALPN, session_exchange.clone())
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
        }
    }

    #[instrument(skip_all,fields(node_id = self.get_node_id_fmt()))]
    pub async fn init_connection(&mut self, remote_node_id: NodeId) -> Result<()> {
        let mut conn = Connection::new(
            &self.rtc_config.api,
            self.rtc_config.config.clone(),
            ConnType::Offerer,
            self.session_exchange.clone(),
            self.connections.len(),
        )
        .await;

        conn.set_remote_node_id(remote_node_id).await?;

        //Typical WebRTC steps...
        let dc_rx = conn.create_data_channel().await;
        info!("Created data channel!");
        conn.init_ice_handler().await;
        info!("Listening for ice candidates");
        conn.offer().await?;
        info!("Created offer!");
        match conn.init_remote_handler().await {
            Ok(()) => info!("Succesfully created remote handler"),
            Err(e) => error!("Error creating remote handler {}", e),
        }

        let conn_rx = conn.monitor_connection().await;

        conn.wait_for_data_channel().await;

        let subscriber = ConnSubscriber {
            dc_rx,
            conn_rx,
            conn,
        };

        //Save connection so we can refernce it by index later
        let connections = &mut self.connections;
        connections.push(subscriber);
        Ok(())
    }

    #[instrument(skip_all,fields(node_id = self.get_node_id_fmt()))]
    pub async fn receive_connection(&mut self) -> Result<()> {
        let mut conn = Connection::new(
            &self.rtc_config.api,
            self.rtc_config.config.clone(),
            ConnType::Offerer,
            self.session_exchange.clone(),
            self.connections.len(),
        )
        .await;
        let dc_rx = conn.register_data_channel().await;
        info!("Registered data channel");
        conn.init_ice_handler().await;
        info!("init ice handler");
        conn.init_remote_handler().await?;
        info!("init remote handler");
        conn.get_remote_node_id().await?;
        conn.answer().await?;
        let conn_rx = conn.monitor_connection().await;

        conn.wait_for_data_channel().await;

        let subscriber = ConnSubscriber {
            dc_rx,
            conn_rx,
            conn,
        };

        //Save connection so we can refernce it by index later
        let connections = &mut self.connections;
        connections.push(subscriber);

        Ok(())
    }

    //TODO: use tokio select to monitor each channel in connections vec
    pub fn run(&self) {}

    pub fn get_node_id(&self) -> NodeId {
        self.node.node_id()
    }

    fn get_node_id_fmt(&self) -> String {
        let node_id = self.node.node_id().fmt_short();
        node_id.clone()
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
