use crate::core::rtc::Connection;
use crate::debug::TEST_ROOT;
use crate::utils::{
    constants::STUN_SERVERS,
    enums::{ConnType, SessionType},
};

use iroh::{
    blobs::store::fs::Store,
    node::{self, Node},
};
use serde::{Deserialize, Serialize};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription,
    },
};

struct RTCConfig {
    api: webrtc::api::API,
    config: RTCConfiguration,
}

//Session information that is stored in a iroh blob
#[derive(Deserialize, Serialize)]
pub struct Session {
    session_type: SessionType,
    local_sdp: Option<RTCSessionDescription>,
}

pub struct Client {
    pub connections: Vec<Connection>,
    pub node: Node<Store>,
    session: Session,
    rtc_config: RTCConfig,
}

impl Client {
    pub async fn new() -> Self {
        let node = node::Node::persistent(TEST_ROOT)
            .await
            .expect("Error creating node");
        let node = node.spawn().await.expect("Error spawning node");
        let session = Session {
            session_type: SessionType::Idle,
            local_sdp: None,
        };

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
        println!("We are : {}", node.node_id());
        let endpoint = node.endpoint();
        //TODO: set up endpoint, signaler, and listner
        endpoint.direct_addresses();
        Client {
            connections: Vec::new(),
            node,
            session,
            rtc_config: RTCConfig { api, config },
        }
    }

    pub async fn init_connection(&self, session_type: SessionType) {
        let conn = Connection::new(
            &self.rtc_config.api,
            self.rtc_config.config.clone(),
            ConnType::Offerer,
        )
        .await;

        conn.create_data_channel().await;
        conn.init_ice_handler().await;
        let offer = match conn.offer().await {
            Ok(offer) => offer,
            Err(e) => panic!("{e}"),
        };
        let session = Session {
            local_sdp: Some(offer),
            session_type,
        };
        let bytes = serde_json::to_vec(&session).expect("Failed to serialize sdp");
        let client = self.node.blobs();
        let hash = &self.node.blobs().add_bytes(bytes).await;
        //let answer = debug::get_sdp(&conn.conn_type);
        //conn.set_remote(answer).await;
    }

    pub async fn answer(&self) {}
}
