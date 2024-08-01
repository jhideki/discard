mod utils {
    pub mod constants;
    pub mod debug;
    pub mod enums;
    pub mod logger;
    pub mod types;
}
#[cfg(test)]
mod tests {
    pub mod client;
}
mod core {
    pub mod client;
    pub mod rtc;
    pub mod signal;
}
mod db {
    pub mod data;
    pub mod db;
}
use clap::{Arg, Command};
use core::client;
use utils::debug;
use utils::enums::ConnType;
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
    /*let mut m = MediaEngine::default();
    let _ = m.register_default_codecs();

    let mut registry = Registry::new();
    registry =
        register_default_interceptors(registry, &mut m).expect("Failed to regiestire interceptors");
    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    //TEMP - used for testing offerer + answerer
    //TODO: create client module to set up connections between peers
    let conn_type = match *is_offerer {
        true => ConnType::Offerer,
        false => ConnType::Answerer,
    };
    let conn = rtc::Connection::new(&api, config.clone(), conn_type.clone()).await;
    if conn_type == ConnType::Offerer {
        conn.create_data_channel().await;
        conn.init_ice_handler().await;
        conn.offer().await;
        let answer = debug::get_sdp(&conn.conn_type);
        conn.set_remote(answer).await;
    } else {
        conn.register_data_channel().await;
        conn.init_ice_handler().await;
        let offer = debug::get_sdp(&conn.conn_type);
        conn.set_remote(offer).await;
        conn.answer().await;
    }*/

    //Keep connection alive
    //conn.monitor_connection().await;
}
