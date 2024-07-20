use webrtc::{
    api::API,
    peer_connection::{configuration::RTCConfiguration, RTCPeerConnection},
};

pub struct Local {
    peer_connection: RTCPeerConnection,
}

impl Local {
    pub async fn new(api: &API, config: RTCConfiguration) -> Self {
        let peer_connection = api
            .new_peer_connection(config)
            .await
            .expect("Failed to establish pc");
        Self { peer_connection }
    }

    pub async fn offer(&self) {
        let offer = self
            .peer_connection
            .create_offer(None)
            .await
            .expect("Error creating offer");

        let offer = serde_json::to_string(&offer).expect("Error serializing offer sdp");
        println!("{}", offer);
    }
}

pub struct Remote {
    peer_connection: RTCPeerConnection,
}
impl Remote {
    pub async fn new(api: &API, config: RTCConfiguration) -> Self {
        let peer_connection = api
            .new_peer_connection(config)
            .await
            .expect("Failed to establish pc");
        Self { peer_connection }
    }

    pub async fn answer(&self) {
        let offer = self
            .peer_connection
            .create_answer(None)
            .await
            .expect("Failed to create answer");

        let answer = serde_json::to_string(&offer).expect("Error serializing offer sdp");
        println!("{}", answer);
    }
}
