use std::sync::Arc;

use anyhow::Result;
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use lazy_static::lazy_static;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;

lazy_static! {
    static ref PEER_CONNECTION_MUTEX: Arc<Mutex<Option<Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(None));
    static ref PENDING_CANDIDATES: Arc<Mutex<Vec<RTCIceCandidate>>> =
        Arc::new(Mutex::new(Vec::new()));
    static ref ADRESS: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
}

async fn signal_candidate(addr: &str, c: &RTCIceCandidate) -> Result<()> {
    let payload = c.to_json()?.candidate;
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("http://{addr}/candidate"))
        .header("content-type", "application/josn; charset=utf-8")
        .body(Bytes::from(payload));
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    let _res = sender.send_request(req).await?;

    Ok(())
}
async fn remote_handler(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let pc = {
        let pcm = PEER_CONNECTION_MUTEX.lock().await;
        pcm.clone().unwrap()
    };
    let addr = {
        let addr = ADRESS.lock().await;
        addr.clone()
    };

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/candidate") => {
            let cand = match std::str::from_utf8(&req.collect().await?.to_bytes()) {
                Ok(s) => s,
                Err(e) => panic!("{e}"),
            };
        }
    }
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
