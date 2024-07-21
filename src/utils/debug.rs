use std::io::{self, Write};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::core::rtc::ConnType;

pub fn get_user_input(conn_type: &ConnType) -> RTCSessionDescription {
    let mut input = String::new();
    match conn_type {
        ConnType::Offerer => println!("Enter answer:"),
        ConnType::Answerer => println!("Enter offer:"),
    }
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    let sdp: RTCSessionDescription = serde_json::from_str(input).expect("Error deserializing sdp");
    sdp
}
