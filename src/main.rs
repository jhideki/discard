use core::rtc;

use utils::debug;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::configuration::RTCConfiguration,
};

mod utils {
    pub mod debug;
    pub mod lib;
}
mod core {
    pub mod rtc;
}
#[tokio::main]
async fn main() {
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
    let local = rtc::Local::new(&api, config.clone()).await;
    local.offer().await;
    let answer = debug::get_user_input();
}
