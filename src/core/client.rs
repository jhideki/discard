use crate::core::rtc::Connection;
use crate::core::signal::{Session, SessionExchange};
use crate::debug::TEST_ROOT;
use crate::utils::{
    constants::{SDP_ALPN, STUN_SERVERS},
    enums::{ConnType, SessionType},
};

use anyhow::Result;
use iroh::net::NodeId;
use iroh::{
    base::key::PublicKey,
    blobs::store::fs::Store,
    node::{self, Builder, Node},
};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::configuration::RTCConfiguration,
};

use std::sync::Arc;

struct RTCConfig {
    api: webrtc::api::API,
    config: RTCConfiguration,
}

//Node information to hand to peer 2 via other means.
struct UserId {
    node_id: PublicKey,
}

pub struct Client {
    pub connections: Vec<Connection>,
    rtc_config: RTCConfig,
    node: Node<Store>,
    session_exchange: Arc<SessionExchange>,
}

impl Client {
    pub async fn new() -> Self {
        let builder = Builder::default()
            .persist(TEST_ROOT)
            .await
            .expect("Failed to create store")
            .disable_docs()
            .build()
            .await
            .expect("Failed to build node");

        let proto = SessionExchange::new(builder.endpoint().clone());

        let node = builder
            .accept(SDP_ALPN, proto.clone())
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
            rtc_config: RTCConfig { api, config },
            node,
            session_exchange: proto,
        }
    }

    pub async fn init_connection(&self) -> Result<()> {
        //TODO: get remote node_id
        let mut conn = Connection::new(
            &self.rtc_config.api,
            self.rtc_config.config.clone(),
            ConnType::Offerer,
            self.session_exchange.clone(),
            self.node.node_id(), //temp
        )
        .await;

        //Typical WebRTC steps...
        conn.create_data_channel().await;
        conn.init_ice_handler().await;
        conn.offer().await;
        conn.get_remote().await;
        conn.monitor_connection().await;

        //Returns handles to worker threads
        let handles = conn.get_task_handles();
        Ok(())
    }
}
