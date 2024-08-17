use crate::core::rtc::{APIWrapper, Connection, RTCConfigurationWrapper};
use crate::core::signal::{SessionExchange, Signaler};
use crate::database::{
    db::Database,
    models::{Message, User},
};

use crate::utils::enums::UserStatus;
use crate::utils::{
    constants::{
        SDP_ALPN, SEND_TEXT_MESSAGE_DELAY, SEND_TEXT_MESSAGE_TIMEOUT, SIGNAL_ALPN, STUN_SERVERS,
        TEST_DB_ROOT,
    },
    enums::{ConnType, MessageType, RunMessage, SignalMessage},
    types::{NodeId, TextMessage},
};

use anyhow::Result;
use futures::stream;
use iroh::{
    blobs::store::fs::Store,
    node::{Builder, Node},
};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{sleep, timeout, Duration};
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

use futures::stream::StreamExt;
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
            node_id: serde_json::to_string(&node.node_id()).unwrap(),
            status: UserStatus::Online,
        };

        match db.write_user(user) {
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

    pub fn store_message(&mut self, message: TextMessage) -> Result<()> {
        let db = &mut self.db;

        let message = Message {
            message_id: 1,
            content: message.content,
            sender_node_id: serde_json::to_string(&self.node.node_id())?,
            sent_ts: Some(message.timestamp.to_string()),
            read_ts: None,
            received_ts: None,
        };
        match db.write_message(message) {
            Ok(()) => info!("Succesfully wrote message to db"),
            Err(e) => error!("Error writing message to db. Error msg: {}", e),
        }
        Ok(())
    }

    pub fn get_node_id(&self) -> NodeId {
        self.node.node_id()
    }

    fn get_node_id_fmt(&self) -> String {
        let node_id = self.node.node_id().fmt_short();
        node_id.clone()
    }

    pub async fn send_message(&mut self, conn_id: usize, message: TextMessage) -> Result<()> {
        let conn = &self.connections[conn_id];
        match timeout(Duration::from_secs(SEND_TEXT_MESSAGE_TIMEOUT), async {
            loop {
                match conn.send_dc_message(message.content.clone()).await {
                    Ok(_) => break,
                    Err(e) => error!("Error sending text message {}", e),
                }

                sleep(Duration::from_secs(SEND_TEXT_MESSAGE_DELAY)).await;
            }
        })
        .await
        {
            Ok(_) => info!("Succesfully sent text message"),
            Err(_) => error!("Failed to send message. Will try again when peer is online"),
        }
        let db = &mut self.db;

        let message = Message {
            message_id: 1,
            content: message.content,
            sender_node_id: serde_json::to_string(&self.node.node_id())?,
            sent_ts: Some(message.timestamp.to_string()),
            read_ts: None,
            received_ts: None,
        };

        match db.write_message(message) {
            Ok(()) => info!("Succesfully wrote message to db"),
            Err(e) => error!("Error writing message to db. Error msg: {}", e),
        }

        Ok(())
    }
}

//Main runtime loop of backend
//TODO: establish audio stream connections + file transmition
pub async fn run(
    client: Client,
    tx: mpsc::Sender<RunMessage>,
    mut rx: mpsc::Receiver<RunMessage>,
) -> Result<()> {
    //Pass sender so that the signaler can signal when an peer wants to establish a connection
    client.signaler.init_sender(tx.clone()).await;
    let client = Arc::new(Mutex::new(client));
    while let Some(message) = rx.recv().await {
        match message {
            RunMessage::ReceiveMessage => {
                let client = Arc::clone(&client);
                let handle = tokio::spawn(receive_connection(client));
            }
            RunMessage::SendMessage((node_id, message)) => {
                let client = Arc::clone(&client);
                let client2 = Arc::clone(&client);

                //Retreive connection id once connection is established
                let (tx, rx) = oneshot::channel();

                let handle = tokio::spawn(init_connection(client, node_id, tx));
                let conn_id = rx.await?;

                let mut client = client2.lock().await;
                //Send message after connection is established
                client.send_message(conn_id, message).await?;
            }
            RunMessage::Online => info!("Peer is online!"),
        }
    }
    Ok(())
}

pub async fn init_connection(
    client: Arc<Mutex<Client>>,
    remote_node_id: NodeId,
    sender: oneshot::Sender<usize>,
) -> Result<()> {
    //Initialize the connection then drop the mutex on client
    let mut conn = {
        let client = client.lock().await;
        let conn = Connection::new(
            &client.rtc_config.api,
            client.rtc_config.config.clone(),
            ConnType::Offerer,
            client.session_exchange.clone(),
            client.connections.len(),
        )
        .await;
        conn
    };

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
    {
        let mut client = client.lock().await;
        let connections = &mut client.connections;
        connections.push(conn);

        let id = connections.len() - 1;
        match sender.send(id) {
            Ok(_) => {}
            Err(e) => error!("Error sending conn id {}", e),
        };
    }

    run_connection(Arc::clone(&client), vec![dc_rx, conn_rx]).await;
    Ok(())
}

pub async fn receive_connection(client: Arc<Mutex<Client>>) -> Result<()> {
    let mut conn = {
        let client = client.lock().await;
        let conn = Connection::new(
            &client.rtc_config.api,
            client.rtc_config.config.clone(),
            ConnType::Offerer,
            client.session_exchange.clone(),
            client.connections.len(),
        )
        .await;
        conn
    };
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
    {
        let mut client = client.lock().await;
        let connections = &mut client.connections;
        connections.push(conn);
    }

    run_connection(Arc::clone(&client), vec![dc_rx, conn_rx]).await;
    Ok(())
}

pub async fn run_connection(
    client: Arc<Mutex<Client>>,
    receivers: Vec<mpsc::Receiver<MessageType>>,
) {
    let streams: Vec<_> = receivers.into_iter().map(ReceiverStream::new).collect();
    let mut fused_streams = stream::select_all(streams);

    loop {
        tokio::select! {
            Some(msg) = fused_streams.next() => {
                match msg{
                    MessageType::Message(m) => { info!("Recieved message: {}", m);
                        let mut client = client.lock().await;
                        client.store_message(m);
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
