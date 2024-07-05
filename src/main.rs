mod body;

use std::sync::Arc;

use anyhow::Result;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::{Body, Bytes, Frame};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use lazy_static::lazy_static;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

lazy_static! {
    static ref PEER_CONNECTION_MUTEX: Arc<Mutex<Option<Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(None));
    static ref PENDING_CANDIDATES: Arc<Mutex<Vec<RTCIceCandidate>>> =
        Arc::new(Mutex::new(Vec::new()));
    static ref ADRESS: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
}

async fn signal_candidate<B>(addr: &str, c: &RTCIceCandidate) -> Result<()> {
    let payload = c.to_json()?.candidate;
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("http://{addr}/candidate"))
        .header("content-type", "application/json; charset=utf-8")
        .body(Body)?;
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
            let candidate = match std::str::from_utf8(&req.collect().await?.to_bytes()) {
                Ok(s) => s.to_owned(),
                Err(e) => panic!("{e}"),
            };
            if let Err(e) = pc
                .add_ice_candidate(RTCIceCandidateInit {
                    candidate,
                    ..Default::default()
                })
                .await
            {
                panic!("{e}");
            }
            let mut response = Response::new(empty());
            *response.status_mut() = StatusCode::OK;
            Ok(response)
        }
        (&Method::POST, "/sdp") => {
            let sdp_str = match std::str::from_utf8(&req.collect().await?.to_bytes()) {
                Ok(s) => s.to_owned(),
                Err(e) => panic!("{e}"),
            };

            let sdp = match serde_json::from_str::<RTCSessionDescription>(&sdp_str) {
                Ok(s) => s,
                Err(e) => panic!("{e}"),
            };

            let answer = match pc.create_answer(None).await {
                Ok(a) => a,
                Err(e) => panic!("{e}"),
            };

            let payload = match serde_json::to_string(&answer) {
                Ok(p) => p,
                Err(e) => panic!("{e}"),
            };

            let req = Request::builder()
                .method(Method::POST)
                .uri(format!("http://{addr}/sdp"))
                .header("content-type", "application/josn; charset=utf-8")
                .body(Bytes::from(payload));

            let stream = match TcpStream::connect(addr).await {
                Ok(s) => s,
                Err(e) => panic!("{e}"),
            };

            let io = TokioIo::new(stream);

            let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
                Ok((s, c)) => (s, c),
                Err(e) => panic!("{e}"),
            };

            let _res = match sender.send_request(req).await {
                Ok(res) => res,
                Err(e) => {
                    println!("{e}");
                    return Err(e);
                }
            };

            if let Err(e) = pc.set_local_description(answer).await {
                panic!("{e}");
            }

            {
                let cs = PENDING_CANDIDATES.lock().await;
                for c in &*cs {
                    if let Err(e) = signal_candidate(&addr, c).await {
                        panic!("{e}");
                    }
                }
            }
            let mut response = Response::new(empty());
            *response.status_mut() = StatusCode::OK;
            Ok(response)
        }
    }
}
fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
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
