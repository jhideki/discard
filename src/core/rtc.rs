use crate::core::signal::{Session, SessionExchange};
use crate::debug;
use crate::utils::enums::ConnType;
use anyhow::{Context, Result};
use iroh::net::key::PublicKey;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::task::JoinHandle;
use tracing::{error, info};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::{
    api::API,
    data_channel::data_channel_message::DataChannelMessage,
    data_channel::RTCDataChannel,
    ice_transport::ice_candidate::RTCIceCandidate,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
};

pub struct APIWrapper(pub API);
impl std::fmt::Debug for APIWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebRTC API struct")
    }
}

#[derive(Clone)]
pub struct RTCConfigurationWrapper(pub RTCConfiguration);
impl std::fmt::Debug for RTCConfigurationWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebRTC Config struct")
    }
}

pub struct RTCDataChannelWrapper(pub Arc<RTCDataChannel>);
impl std::fmt::Debug for RTCDataChannelWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RTC Data Channel")
    }
}

#[derive(Debug)]
pub struct Connection {
    pub peer_connection: Arc<RTCPeerConnection>,
    pub conn_type: ConnType,
    candidates: Arc<Mutex<Vec<RTCIceCandidate>>>,
    pub ice_notify: Arc<Notify>,
    signaler: Arc<SessionExchange>,
    remote_node_id: PublicKey,
    task_handles: Vec<JoinHandle<()>>,
    data_channel: Option<RTCDataChannelWrapper>,
    id: usize,
    message_queue: Arc<Mutex<VecDeque<String>>>,
}

impl Connection {
    pub async fn new(
        api: &APIWrapper,
        config: RTCConfigurationWrapper,
        conn_type: ConnType,
        signaler: Arc<SessionExchange>,
        remote_node_id: PublicKey,
        id: usize,
    ) -> Self {
        let api = &api.0;
        let config = config.0;
        let peer_connection = api
            .new_peer_connection(config)
            .await
            .expect("Failed to establish pc");
        Self {
            peer_connection: Arc::new(peer_connection),
            conn_type,
            candidates: Arc::new(Mutex::new(Vec::new())),
            ice_notify: Arc::new(Notify::new()),
            signaler,
            remote_node_id,
            task_handles: Vec::new(),
            data_channel: None,
            id,
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn monitor_connection(&mut self) {
        let (done_tx, mut done_rx) = mpsc::channel::<()>(10);
        let pc = Arc::clone(&self.peer_connection);
        pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            match s {
                RTCPeerConnectionState::New => info!("New connection"),
                RTCPeerConnectionState::Failed => {
                    info!("Failed connection");
                    let _ = done_tx.send(());
                }
                RTCPeerConnectionState::Closed => info!("Closed connection"),
                RTCPeerConnectionState::Connected => info!("Connected!"),
                RTCPeerConnectionState::Connecting => info!("Connecting..."),
                RTCPeerConnectionState::Unspecified => info!("Unspecified?"),
                _ => info!("???"),
            }
            Box::pin(async {})
        }));

        let handle = tokio::spawn(async move {
            loop {
                if let _ = done_rx.recv().await {
                    info!("Conn discconeted");
                    break;
                }
            }
        });
        self.task_handles.push(handle);
    }

    pub async fn init_ice_handler(&self) {
        let candidates = Arc::clone(&self.candidates);
        let pc = Arc::clone(&self.peer_connection);
        let signaler = Arc::clone(&self.signaler);
        let pc2 = Arc::downgrade(&self.peer_connection);
        let remote_node_id = self.remote_node_id.clone();

        pc.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
            let candidates = Arc::clone(&candidates);
            let pc = pc2.clone();
            let signaler = Arc::clone(&signaler);

            Box::pin(async move {
                if let Some(candidate) = c {
                    let mut candidates = candidates.lock().await;
                    candidates.push(candidate.clone());
                    let sdp = match pc.upgrade() {
                        Some(pc) => pc.local_description().await,
                        None => None,
                    };
                    //Send ice candidate and sdp to peer 2
                    let _ = signaler
                        .send_session(
                            remote_node_id,
                            Session {
                                ice_candidate: Some(candidate.clone()),
                                sdp,
                            },
                        )
                        .await;
                }
            })
        }));
    }

    pub async fn offer(&self) -> Result<RTCSessionDescription> {
        let pc = Arc::clone(&self.peer_connection);
        let offer = pc.create_offer(None).await.expect("Error creating offer");

        if let Err(e) = pc.set_local_description(offer).await {
            panic!("Error setting local offer {}", e);
        }
        let offer = pc
            .local_description()
            .await
            .context("Failed to retreive local sdp from offerer")?;
        Ok(offer)
    }

    //Initializes a listener that receives remote SDPs and ICE candidates
    pub async fn init_remote_handler(&mut self) -> Result<()> {
        let signaler = Arc::clone(&self.signaler);
        let pc = Arc::clone(&self.peer_connection);

        let (tx, mut rx) = mpsc::channel::<Session>(10);
        signaler.init(tx.clone()).await;

        //Spawn a listener to retreive session from remote
        let handle = tokio::spawn(async move {
            //Continue listening for incoming sdps incase connection is reset
            while let Some(session) = rx.recv().await {
                if let Some(sdp) = session.sdp {
                    if let Err(e) = pc.set_remote_description(sdp).await {
                        info!("Error setting sdp {e}");
                    }
                }
                if let Some(candidate) = session.ice_candidate {
                    let c = candidate.to_string();
                    if let Err(e) = pc
                        .add_ice_candidate(RTCIceCandidateInit {
                            candidate: c,
                            ..Default::default()
                        })
                        .await
                    {
                        info!("Error adding ice canddiate {e}");
                    }
                }
            }
        });
        self.task_handles.push(handle);
        Ok(())
    }

    pub async fn answer(&self) {
        let pc = Arc::clone(&self.peer_connection);
        let answer: RTCSessionDescription = pc
            .create_answer(None)
            .await
            .expect("Failed to create answer");

        if let Err(e) = pc.set_local_description(answer).await {
            panic!("Error setting local desc: {}", e);
        }
    }

    pub async fn create_data_channel(&mut self) {
        let pc = Arc::clone(&self.peer_connection);
        let data_channel = pc
            .create_data_channel("messaging", None)
            .await
            .expect("Error creating data channel");

        let dc = Arc::clone(&data_channel);
        data_channel.on_open(Box::new(move || {
            info!("Data channel {} {} is now open", dc.label(), dc.id());
            Box::pin(async move {})
        }));

        let d_label = data_channel.label().to_owned();
        data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
            let message = String::from_utf8(msg.data.to_vec()).expect("Error parsing message");
            info!("Message from peer, {}: {}", d_label, message);
            Box::pin(async {})
        }));
        self.data_channel = Some(RTCDataChannelWrapper(Arc::clone(&data_channel)));
    }

    pub async fn register_data_channel(&self) {
        let pc = Arc::clone(&self.peer_connection);
        let mq = Arc::clone(&self.message_queue);
        pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
            info!("Data channel established from answerer");
            let d2 = Arc::clone(&d);
            let mq = Arc::clone(&mq);
            Box::pin(async move {
                d.on_open(Box::new(move || {
                    info!("Data channel {} {} is now open", d2.label(), d2.id());
                    Box::pin(async move {
                        info!("Getting message");
                        let message = debug::get_message();
                        if let Err(e) = d2.send_text(message).await {
                            info!("Error sending message {}", e);
                        }
                    })
                }));

                d.on_message(Box::new(move |msg: DataChannelMessage| {
                    let message =
                        String::from_utf8(msg.data.to_vec()).expect("Error parsing message");
                    info!("Message from peer: {}", message);

                    let message_queue = Arc::clone(&mq);
                    Box::pin(async move {
                        let mut mq = message_queue.lock().await;
                        mq.push_back(message);
                    })
                }));
            })
        }));
    }

    pub async fn add_ice_candidate(&mut self, candidate: RTCIceCandidate) {
        let candidates = Arc::clone(&self.candidates);
        let mut c = candidates.lock().await;
        c.push(candidate);
    }

    //TODO: Update handles to be a tuple of task_type, handle
    pub fn get_task_handles(&self) -> &Vec<JoinHandle<()>> {
        &self.task_handles
    }

    pub async fn send_dc_message(&self, message: String) -> Result<()> {
        if let Some(dc) = &self.data_channel {
            let data_channel = dc.0.clone();
            data_channel.send_text(message).await?;
        } else {
            error!("Data channel has not been set");
        }
        Ok(())
    }

    //Gets the most recent message received
    pub async fn get_message(&self) -> Option<String> {
        let mq = Arc::clone(&self.message_queue);
        let mut message_queue = mq.lock().await;
        message_queue.pop_front()
    }
}
