use std::sync::Arc;

use anyhow::Result;
use lazy_static::lazy_static;
use reqwest::{Body, Client};
use tokio::sync::Mutex;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;

lazy_static! {
    static ref PEER_CONNECTION_MUTEX: Arc<Mutex<Option<Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(None));
}

async fn signal_candidate(addr: &str, c: &RTCIceCandidate) -> Result<()> {
    let payload = c.to_json()?.candidate;
    let client = Client::new();
    let body = Body::from(payload);
    let _res = match client
        .post(format!("http://{addr}/candidate"))
        .header("content-type", "application/json; charset=utf-8")
        .body(body)
        .send()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            println!("{e}");
            return Err(e.into());
        }
    };

    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ice_server = RTCIceServer {
        urls: vec!["stun:stun.l.google.com:19302".to_owned()],
        ..Default::default()
    };
    let config = RTCConfiguration {
        ice_servers: vec![ice_server],
        ..Default::default()
    };
    let api = APIBuilder::new().build();
    let peer_connection = api.new_peer_connection(config).await?;

    let offer = peer_connection.create_offer(None).await?;
    peer_connection.set_local_description(offer.clone()).await?;

    peer_connection
        .on_ice_candidate(Box::new(|candidate| {
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    println!("ICE Candidate: {:?}", candidate.candidate);
                }
            })
        }))
        .await;
    Ok(())
}
