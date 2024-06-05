use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;

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
