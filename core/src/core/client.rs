use crate::core::ipc::{IPCMessage, IPCResponse, SendUsersResp};
use crate::core::rtc::{APIWrapper, Connection, RTCConfigurationWrapper};
use crate::core::signal::{SessionExchange, Signaler};
use crate::database::{
    db::Database,
    models::{FromRow, Message, User},
};

use crate::utils::enums::{SessionType, UserStatus};
use crate::utils::{
    constants::{
        SDP_ALPN, SEND_TEXT_MESSAGE_DELAY, SEND_TEXT_MESSAGE_TIMEOUT, SIGNAL_ALPN, STUN_SERVERS,
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
use tracing::{error, info};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::configuration::RTCConfiguration,
};

use futures::stream::StreamExt;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug)]
struct RTCConfig {
    api: APIWrapper,
    config: RTCConfigurationWrapper,
}

#[derive(Debug)]
pub struct Client {
    connections: HashMap<String, Connection>,
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
        let db = {
            //TODO: figure out actual persistent location
            let mut db_root = String::from(root);
            db_root.push_str("/.db3");
            match Database::new(&db_root, "./src/database/init.sql") {
                Ok(db) => db,
                Err(e) => panic!(
                    "Error inintializing the database. Exiting the program... Error msg:  {}",
                    e
                ),
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
            connections: HashMap::new(),
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

    pub fn update_status(&mut self, node_id: NodeId, status: UserStatus) -> Result<()> {
        let db = &mut self.db;
        match db.update_status(node_id, status) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Error updating user status: {}", e)),
        }
    }

    pub fn add_user(&mut self, node_id: NodeId, display_name: String) -> Result<()> {
        let db = &mut self.db;
        let serialized_id = serde_json::to_string(&node_id)?;
        let user = User {
            node_id: serialized_id,
            display_name,
            status: UserStatus::Online,
            user_id: 0, //dummy id, wont' actually be 0 in the db
        };
        db.write_user(user)
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

    pub async fn read_messages(&mut self, node_id: NodeId) -> Result<Vec<Message>> {
        let db = &mut self.db;
        let conn = db.get_conn();
        let serialized_id = serde_json::to_string(&node_id).expect("error serializing node id");
        let mut stmt = conn.prepare("select * from users where node_id = ?1")?;

        let msg_iter = stmt.query_map([serialized_id], |row| Message::from_row(row))?;
        let messages = msg_iter.map(|m| m.unwrap());

        Ok(messages.collect())
    }

    pub fn get_user_node_id(&self, display_name: String) -> Result<NodeId> {
        let db = &self.db;
        let conn = db.get_conn();
        let node_id: String = conn.query_row(
            "select node_id from users where display_name = ?1 fetch first 1 row only",
            [display_name],
            |row| row.get(0),
        )?;
        let node_id: NodeId = serde_json::from_str(&node_id)?;
        Ok(node_id)
    }

    pub fn get_users(&self) -> Result<Vec<User>> {
        let db = &self.db;
        let conn = db.get_conn();
        let mut stmt = conn.prepare("select * from users")?;
        let user_iter = stmt.query_map([], |row| User::from_row(row))?;
        let users = user_iter.map(|m| m.unwrap());
        Ok(users.collect())
    }

    pub fn get_display_name(&self, node_id: String) -> Result<String> {
        let db = &self.db;
        let conn = db.get_conn();
        let display_name: String = conn.query_row(
            "select display_name from users where node_id = ?1 fetch first 1 row only",
            [node_id],
            |row| row.get(0),
        )?;
        Ok(display_name)
    }
}

//Main runtime loop of backend
//TODO: establish audio stream connections + file transmition
pub async fn run(
    client: Client,
    tx: mpsc::Sender<RunMessage>,
    mut rx: mpsc::Receiver<RunMessage>,
    data_tx: mpsc::Sender<IPCResponse>,
) -> Result<()> {
    info!("Client is running...");
    //Pass sender so that the signaler can signal when an peer wants to establish a connection
    client.signaler.init_sender(tx.clone()).await;
    let client = Arc::new(Mutex::new(client));
    while let Some(message) = rx.recv().await {
        match message {
            RunMessage::RecvConn(session_type) => {
                info!("Run message received");
                let client = Arc::clone(&client);
                let handle = tokio::spawn(receive_connection(client, SessionType::Chat));
            }
            RunMessage::InitConn(session_type, display_name) => {
                let client = Arc::clone(&client);
                let client2 = Arc::clone(&client);

                let mut client2 = client2.lock().await;

                let node_id = client2.get_user_node_id(display_name)?;

                let handle =
                    tokio::spawn(init_connection(client, node_id, display_name, session_type));
                let conn_id = rx.await?;
            }
            //Assumes connection is already established
            RunMessage::SendMessage(message) => {
                //Send message after connection is established
                client2.send_message(conn_id, message).await?;
            }
            RunMessage::UpdateStatus(node_id, user_status) => {
                let client = Arc::clone(&client);
                let mut client = client.lock().await;
                match client.update_status(node_id, user_status) {
                    Ok(()) => info!("Succesfully updated status "),
                    Err(e) => error!("Failed to update status {}", e),
                }
                info!("Peer is online!");
            }
            RunMessage::Adduser(node_id, display_name) => {
                let client = Arc::clone(&client);
                let mut client = client.lock().await;
                match client.add_user(node_id, display_name) {
                    Ok(()) => info!("Succesfully added a user"),
                    Err(e) => error!("Failed to add user {}", e),
                }
            }
            RunMessage::GetUsers => {
                let client = Arc::clone(&client);
                let client = client.lock().await;
                let users = client.get_users()?;
                let response = SendUsersResp { users };
                data_tx.send(IPCResponse::SendUsers(response)).await?;
            }
            RunMessage::Shutdown => {
                info!("Shutting down...");
                break;
            }
            RunMessage::AudioStream => {
                info!("Creating audio stream connection...");
                let client = Arc::clone(&client);
                let mut client = client.lock().await;
                let node_id = client.get_user_node_id(display_name).await;
            }
        }
    }

    Ok(())
}

pub async fn init_connection(
    client: Arc<Mutex<Client>>,
    remote_node_id: NodeId,
    display_name: String,
    session_type: SessionType,
) -> Result<()> {
    //Initialize the connection then drop the mutex on client
    let mut conn = {
        let client = client.lock().await;
        let conn = Connection::new(
            &client.rtc_config.api,
            client.rtc_config.config.clone(),
            ConnType::Offerer,
            client.session_exchange.clone(),
        )
        .await;
        conn
    };

    conn.set_remote_node_id(remote_node_id).await?;

    let mut receivers: Vec<mpsc::Receiver<MessageType>> = Vec::new();

    match session_type {
        SessionType::Idle => {}
        SessionType::Chat => {
            let dc_rx = conn.create_data_channel().await;
            receivers.push(dc_rx);
        }
        SessionType::Call => {}
        SessionType::Video => {}
    }

    //Typical WebRTC steps...
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
    receivers.push(conn_rx);

    conn.wait_for_data_channel().await;

    //Save connection so we can refernce it by peer's display_name later
    {
        let mut client = client.lock().await;
        let connections = &mut client.connections;
        connections.insert(display_name, conn);
    }

    run_connection(Arc::clone(&client), receivers).await;
    Ok(())
}

pub async fn receive_connection(
    client: Arc<Mutex<Client>>,
    session_type: SessionType,
) -> Result<()> {
    let mut conn = {
        let client = client.lock().await;
        let conn = Connection::new(
            &client.rtc_config.api,
            client.rtc_config.config.clone(),
            ConnType::Offerer,
            client.session_exchange.clone(),
        )
        .await;
        conn
    };

    let mut receivers: Vec<mpsc::Receiver<MessageType>> = Vec::new();

    match session_type {
        SessionType::Idle => {}
        SessionType::Chat => {
            let dc_rx = conn.register_data_channel().await;
            receivers.push(dc_rx);
            info!("Registered data channel");
        }
        SessionType::Call => {}
        SessionType::Video => {}
    }

    conn.init_ice_handler().await;
    info!("init ice handler");
    conn.init_remote_handler().await?;
    info!("init remote handler");
    conn.retrieve_remote_node_id().await?;
    conn.answer().await?;
    let conn_rx = conn.monitor_connection().await;
    receivers.push(conn_rx);

    info!("Connection is running");
    conn.wait_for_data_channel().await;

    //Save connection so we can refernce it by index later
    {
        let mut client = client.lock().await;
        let connections = &mut client.connections;
        if let Ok(remote_node_id) = conn.get_remote_node_id().await {
            let remote_node_id_str = remote_node_id.to_string();
            let display_name = client.get_display_name(remote_node_id_str)?;
            connections.insert(display_name, conn);
        }
    }

    run_connection(Arc::clone(&client), receivers).await;
    Ok(())
}

pub async fn init_call(
    client: Arc<Mutex<Client>>,
    remote_node_id: NodeId,
    sender: oneshot::Sender<usize>,
) {
}
pub async fn recieve_call() {}

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
                        let _ = client.store_message(m);
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
