use crate::core::rtc::{APIWrapper, Connection, RTCConfigurationWrapper};
use crate::core::signal::{SessionExchange, Signaler};
use crate::database::{
    db::Database,
    models::{Message, User},
};

use crate::utils::enums::SignalMessage;
use crate::utils::{
    constants::{SDP_ALPN, SIGNAL_ALPN, STUN_SERVERS, TEST_DB_ROOT},
    enums::{ConnType, MessageType},
    types::{NodeId, TextMessage},
};

use anyhow::Result;
use futures::stream;
use iroh::{
    blobs::store::fs::Store,
    node::{Builder, Node},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, instrument};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::configuration::RTCConfiguration,
};

use futures::stream::{select_all, StreamExt};
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug)]
struct RTCConfig {
    api: APIWrapper,
    config: RTCConfigurationWrapper,
}

#[derive(Debug)]
pub struct Client {
    connections: Vec<Connection>,
    rtc_config: RTCConfig,
    node: Node<Store>,
    session_exchange: Arc<SessionExchange>,
    db: Database,
    signaler: Arc<Signaler>,
}

impl Client {
    pub async fn new(root: &str) -> Self {
        //Iroh setup
        let builder = Builder::default()
            .persist(root)
            .await
            .expect("Failed to create store")
            .disable_docs()
            .build()
            .await
            .expect("Failed to build node");

        let session_exchange = SessionExchange::new(builder.endpoint().clone());
        let signaler = Signaler::new(builder.endpoint().clone());
        let node = builder
            .accept(SDP_ALPN, session_exchange.clone())
            .accept(SIGNAL_ALPN, signaler.clone())
            .spawn()
            .await
            .expect("Failed to spawn node");

        //Webrtc setup
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

        //DB setup
        //TODO: change db directory to a proper file location for linux and windows
        let mut db = {
            if cfg!(test) {
                match Database::new(TEST_DB_ROOT, "../database/init.sql") {
                    Ok(db) => db,
                    Err(e) => panic!(
                        "Error inintializing the database. Exiting the program... Error msg:  {}",
                        e
                    ),
                }
            } else {
                //TODO: figure out actual persistent location
                match Database::new("./test-db3", "../database/init.sql") {
                    Ok(db) => db,
                    Err(e) => panic!(
                        "Error inintializing the database. Exiting the program... Error msg:  {}",
                        e
                    ),
                }
            }
        };

        //TODO: change user schema
        let user = User {
            user_id: 1,
            display_name: "test".to_string(),
            node_id: node.node_id().to_string(),
            is_online: true,
        };

        match db.write(&user) {
            Ok(()) => (),
            Err(e) => error!("Error initializing account. Error msg: {}", e),
        }

        Client {
            connections: Vec::new(),
            rtc_config: RTCConfig {
                api: APIWrapper(api),
                config: RTCConfigurationWrapper(config),
            },
            node,
            session_exchange,
            db,
            signaler,
        }
    }

    #[instrument(skip_all,fields(node_id = self.get_node_id_fmt()))]
    pub async fn init_connection(
        &mut self,
        remote_node_id: NodeId,
    ) -> Result<Vec<mpsc::Receiver<MessageType>>> {
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
        info!("Connection is running");

        conn.wait_for_data_channel().await;

        //Save connection so we can refernce it by index later
        let connections = &mut self.connections;
        connections.push(conn);

        Ok(vec![dc_rx, conn_rx])
    }

    #[instrument(skip_all,fields(node_id = self.get_node_id_fmt()))]
    pub async fn receive_connection(&mut self) -> Result<Vec<mpsc::Receiver<MessageType>>> {
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

        info!("Connection is running");
        conn.wait_for_data_channel().await;

        //Save connection so we can refernce it by index later
        let connections = &mut self.connections;
        connections.push(conn);

        Ok(vec![dc_rx, conn_rx])
    }

    pub async fn run_connection(&mut self, receivers: Vec<mpsc::Receiver<MessageType>>) {
        let streams: Vec<_> = receivers.into_iter().map(ReceiverStream::new).collect();
        let mut fused_streams = stream::select_all(streams);
        loop {
            tokio::select! {
                Some(msg) = fused_streams.next() => {
                    match msg{
                        MessageType::Message(m) => { info!("Recieved message: {}", m);
                            self.store_message(m);
                        },
                        MessageType::ConnectionState(_) => info!("Connection state changed"),
                    }
                }
                else => {
                info!("Strems have closed");
                break;
            }
            }
        }
    }

    fn store_message(&mut self, message: TextMessage) {
        let db = &mut self.db;

        let message = Message {
            message_id: 1,
            content: message.content,
            sender_id: 1,
            sent_ts: Some(message.timestamp.to_string()),
            read_ts: None,
            received_ts: None,
        };
        match db.write(&message) {
            Ok(()) => info!("Succesfully wrote message to db"),
            Err(e) => error!("Error writing message to db. Error msg: {}", e),
        }
    }

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
        let db = &mut self.db;

        let message = Message {
            message_id: 1,
            content: message,
            sender_id: 1,
            sent_ts: Some(message.timestamp.to_string()),
            read_ts: None,
            received_ts: None,
        };

        match db.write(&message) {
            Ok(()) => info!("Succesfully wrote message to db"),
            Err(e) => error!("Error writing message to db. Error msg: {}", e),
        }
        //TODO: write message to db

        Ok(())
    }
}

//Main runtime loop of backend
pub async fn run(mut client: Client) {
    let (tx, mut rx) = mpsc::channel::<SignalMessage>(10);
    client.signaler.init_sender(tx.clone());
    while let Some(message) = rx.recv().await {
        match message {
            SignalMessage::ReceiveConnection => {
                let handle = tokio::spawn(client.receive_connection());
            }
            SignalMessage::SendConenction => {
                let handle = tokio::spawn(client.init_connection());
            }
            SignalMessage::Online => info!("Peer is online!"),
        }
    }
}
