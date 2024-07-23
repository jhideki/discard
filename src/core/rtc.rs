use crate::debug;
use tokio::sync::{mpsc, Mutex, Notify};
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

use std::sync::Arc;

#[derive(PartialEq, Clone)]
pub enum ConnType {
    Offerer,
    Answerer,
}

pub struct Connection {
    peer_connection: Arc<Mutex<RTCPeerConnection>>,
    pub conn_type: ConnType,
    candidates: Arc<Mutex<Vec<RTCIceCandidate>>>,
    ice_notify: Arc<Notify>,
}

impl Connection {
    pub async fn new(api: &API, config: RTCConfiguration, conn_type: ConnType) -> Self {
        let peer_connection = api
            .new_peer_connection(config)
            .await
            .expect("Failed to establish pc");
        Self {
            peer_connection: Arc::new(Mutex::new(peer_connection)),
            conn_type,
            candidates: Arc::new(Mutex::new(Vec::new())),
            ice_notify: Arc::new(Notify::new()),
        }
    }

    pub async fn monitor_connection(&self) {
        let (done_tx, mut done_rx) = mpsc::channel::<()>(1);
        let pc = Arc::clone(&self.peer_connection);
        {
            let pc = pc.lock().await;
            pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                match s {
                    RTCPeerConnectionState::New => println!("New connection"),
                    RTCPeerConnectionState::Failed => {
                        println!("Failed connection");
                        let _ = done_tx.send(());
                    }
                    RTCPeerConnectionState::Closed => println!("Closed connection"),
                    RTCPeerConnectionState::Connected => println!("Connected!"),
                    RTCPeerConnectionState::Connecting => println!("Connecting..."),
                    RTCPeerConnectionState::Unspecified => println!("Unspecified?"),
                    _ => println!("???"),
                }
                Box::pin(async {})
            }));
        }

        loop {
            println!("Waiting...");
            if let signal = done_rx.recv().await {
                println!("Conn discconeted");
                break;
            }
        }
    }

    pub async fn init_ice_handler(&self) {
        self.print_local_sdp();
        let candidates = Arc::clone(&self.candidates);
        let notify = Arc::clone(&self.ice_notify);
        let pc = Arc::clone(&self.peer_connection);
        let pc = pc.lock().await;
        pc.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
            println!("ICE candidate------------:\n {:?}", c);
            let pc = Arc::clone(&candidates);
            let n = Arc::clone(&notify);
            Box::pin(async move {
                if let Some(candidate) = c {
                    let mut candidates = pc.lock().await;
                    candidates.push(candidate);
                    n.notify_waiters();
                }
            })
        }));
    }

    fn print_local_sdp(&self) {
        let notify = Arc::clone(&self.ice_notify);
        let pc = Arc::clone(&self.peer_connection);
        tokio::spawn(async move {
            loop {
                notify.notified().await;
                println!("Printing sdps...");
                let pc = pc.lock().await;
                let ld = pc
                    .local_description()
                    .await
                    .expect("Error retreiving local description");
                let ld = serde_json::to_string(&ld).expect("Failed to deserialized sdp");
                println!("{}", ld);
            }
        });
    }

    pub async fn offer(&self) {
        let pc = Arc::clone(&self.peer_connection);
        let pc = pc.lock().await;
        let offer = pc.create_offer(None).await.expect("Error creating offer");

        if let Err(e) = pc.set_local_description(offer).await {
            panic!("Error setting local offer {}", e);
        }
    }

    pub async fn set_remote(&self, sdp: RTCSessionDescription) {
        println!("--------------Setting remote--------------");
        let pc = Arc::clone(&self.peer_connection);
        let pc = pc.lock().await;
        if let Err(e) = pc.set_remote_description(sdp).await {
            panic!("Error setting local answer {}", e);
        }
        println!("--------------Done Setting remote--------------");
    }

    pub async fn answer(&self) {
        println!("--------------Creating answer--------------");
        let pc = Arc::clone(&self.peer_connection);
        let pc = pc.lock().await;
        let answer: RTCSessionDescription = pc
            .create_answer(None)
            .await
            .expect("Failed to create answer");

        if let Err(e) = pc.set_local_description(answer).await {
            panic!("Error setting local desc: {}", e);
        }

        println!("--------------Done createing answer--------------");
    }

    pub async fn create_data_channel(&self) {
        let pc = Arc::clone(&self.peer_connection);
        let pc = pc.lock().await;
        let data_channel = pc
            .create_data_channel("messaging", None)
            .await
            .expect("Error creating data channel");

        let dc = Arc::clone(&data_channel);
        data_channel.on_open(Box::new(move || {
            println!("Data channel {} {} is now open", dc.label(), dc.id());
            let dc2 = Arc::clone(&dc);
            Box::pin(async move {
                println!("Getting message");
                let message = debug::get_message();
                if let Err(e) = dc2.send_text(message).await {
                    println!("Error sending message {}", e);
                }
            })
        }));

        let d_label = data_channel.label().to_owned();
        data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
            let message = String::from_utf8(msg.data.to_vec()).expect("Error parsing message");
            println!("Message from peer, {}: {}", d_label, message);
            Box::pin(async {})
        }));
    }

    pub async fn register_data_channel(&self) {
        let pc = Arc::clone(&self.peer_connection);
        let pc = pc.lock().await;
        pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
            println!("Data channel established from answerer");
            let d2 = Arc::clone(&d);
            Box::pin(async move {
                d.on_open(Box::new(move || {
                    println!("Data channel {} {} is now open", d2.label(), d2.id());
                    Box::pin(async move {
                        println!("Getting message");
                        let message = debug::get_message();
                        if let Err(e) = d2.send_text(message).await {
                            println!("Error sending message {}", e);
                        }
                    })
                }));

                d.on_message(Box::new(move |msg: DataChannelMessage| {
                    let message =
                        String::from_utf8(msg.data.to_vec()).expect("Error parsing message");
                    println!("Message from peer: {}", message);
                    Box::pin(async {})
                }));
            })
        }));
    }
}
