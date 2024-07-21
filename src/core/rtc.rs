use crate::debug;
use webrtc::{
    api::API, ice_transport::ice_candidate::RTCIceCandidate, peer_connection::{
        configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    }
};

use std::sync::{Arc,Mutex};


pub enum ConnType {
    Offerer,
    Answerer,
}

pub struct Connection {
    peer_connection: RTCPeerConnection,
    conn_type: ConnType,
    candidates: Arc<Mutex<Vec<RTCIceCandidate>>>,
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
        }
    }

    pub async fn start(&self){
        &self.peer_connection.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>|{
            let pc = Arc::clone(&self.candidates);
            Box::pin(async move{

            if let Some(candidate) = c{
                let candidates = pc.lock().unwrap();
                pc.push(c);
            }
            })
        }))
        
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
        &self.peer_connection.add_ice_candidate
        println!("Offer : {}", offer_json);

        let answer = debug::get_user_input(&self.conn_type);
        if let Err(e) = &self.peer_connection.set_remote_description(answer).await {
            panic!("Error setting local answer {}", e);
        }
    }

    pub async fn answer(&self) {
        let offer = debug::get_user_input(&self.conn_type);
        if let Err(e) = &self.peer_connection.set_remote_description(offer).await {
            panic!("Error setting remote desc: {}", e);
        }

        let answer: RTCSessionDescription = self
            .peer_connection
            .create_answer(None)
            .await
            .expect("Failed to create answer");

        let answer_json = serde_json::to_string(&answer).expect("Error serializing offer sdp");
        if let Err(e) = &self.peer_connection.set_local_description(answer).await {
            panic!("Error setting local desc: {}", e);
        }

        println!("{}", answer_json);
    }

    pub async fn create_data_channel(&self) {
        let data_channel = self
            .peer_connection
            .create_data_channel("messaging", None)
            .await;
    }
}
