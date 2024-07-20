use std::io::{self, Write};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

pub fn get_user_input() -> RTCSessionDescription {
    let mut input = String::new();
    print!("Paste in the remote SDP: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    let sdp: RTCSessionDescription = serde_json::from_str(input).expect("Error deserializing sdp");
    sdp
}
