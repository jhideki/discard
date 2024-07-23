use crate::debug;
use tokio::sync::{mpsc, Notify};
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

use std::sync::{Arc, Mutex};

#[derive(PartialEq, Clone)]
pub enum ConnType {
    Offerer,
    Answerer,
}

pub struct Connection {
    peer_connection: RTCPeerConnection,
    conn_type: ConnType,
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
            peer_connection,
            conn_type,
            candidates: Arc::new(Mutex::new(Vec::new())),
            ice_notify: Arc::new(Notify::new()),
        }
    }

    pub async fn monitor_connection(&self) {
        let (done_tx, mut done_rx) = mpsc::channel::<()>(1);
        let _ = &self
            .peer_connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
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
        loop {
            println!("Waiting...");
            if let signal = done_rx.recv().await {
                println!("Conn discconeted");
                break;
            }
        }
    }

    pub fn init_ice_handler(&self) {
        let candidates = Arc::clone(&self.candidates);
        let notify = Arc::clone(&self.ice_notify);
        let _ =
            &self
                .peer_connection
                .on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
                    println!("ICE candidate------------:\n {:?}", c);
                    let pc = Arc::clone(&candidates);
                    let n = Arc::clone(&notify);
                    Box::pin(async move {
                        if let Some(candidate) = c {
                            let mut candidates = pc.lock().unwrap();
                            candidates.push(candidate);
                            n.notify_one();
                        }
                    })
                }));
    }

    pub async fn offer(&self) {
        let offer = self
            .peer_connection
            .create_offer(None)
            .await
            .expect("Error creating offer");

        let offer_json = serde_json::to_string(&offer).expect("Error serializing offer sdp");
        if let Err(e) = &self.peer_connection.set_local_description(offer).await {
            panic!("Error setting local offer {}", e);
        }
        println!("Offer : {}", offer_json);

        let answer = debug::get_sdp(&self.conn_type);
        if let Err(e) = &self.peer_connection.set_remote_description(answer).await {
            panic!("Error setting local answer {}", e);
        }
    }

    pub async fn answer(&self) {
        let offer = debug::get_sdp(&self.conn_type);
        if let Err(e) = &self.peer_connection.set_remote_description(offer).await {
            panic!("Error setting remote desc: {}", e);
        }

        let answer: RTCSessionDescription = self
            .peer_connection
            .create_answer(None)
            .await
            .expect("Failed to create answer");

        let answer_json = serde_json::to_string(&answer).expect("Error serializing offer sdp");
        println!("{}", answer_json);
        //wait until the offerer sets remote sdp
        debug::wait();
        if let Err(e) = &self.peer_connection.set_local_description(answer).await {
            panic!("Error setting local desc: {}", e);
        }
    }

    pub async fn create_data_channel(&self) {
        let data_channel = &self
            .peer_connection
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
        self.peer_connection
            .on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
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
