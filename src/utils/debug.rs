use std::io::{self, Write};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::core::rtc::ConnType;

pub fn get_sdp(conn_type: &ConnType) -> RTCSessionDescription {
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

pub fn get_message() -> String {
    let mut input = String::new();
    println!("Message: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    input.to_string()
}

pub fn wait() {
    let mut input = String::new();
    println!("Waiting for offerer to set remote pres any key when done...");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
}
