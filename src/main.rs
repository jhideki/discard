mod utils {
    pub mod debug;
    pub mod lib;
}
mod core {
    pub mod rtc;
}

use clap::{Arg, Command};
use core::{rtc, rtc::ConnType};
use utils::debug;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::configuration::RTCConfiguration,
};

#[tokio::main]
async fn main() {
    let matches = Command::new("discard")
        .arg(
            Arg::new("offer")
                .short('o')
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();
    let is_offerer = matches
        .get_one::<bool>("offer")
        .expect("error parsing cli args");
    println!("is_remote: {}", is_offerer);
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut m = MediaEngine::default();
    m.register_default_codecs();

    let mut registry = Registry::new();
    registry =
        register_default_interceptors(registry, &mut m).expect("Failed to regiestire interceptors");
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    if *is_offerer {
        println!("We are the offerer!");
        let local = rtc::Connection::new(&api, config.clone(), ConnType::Offerer).await;
        local.offer().await;
    } else {
        println!("We are the answerer!");
        let remote = rtc::Connection::new(&api, config.clone(), ConnType::Answerer).await;
        remote.answer().await;
    }
}
