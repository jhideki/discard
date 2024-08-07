//WebRTC setup
pub const STUN_SERVERS: [&str; 1] = ["stun:stun.l.google.com:19302"];

//Signal Config
pub const SDP_ALPN: &[u8] = b"discard/sdp-exchange";
pub const ID_APLN: &[u8] = b"discard/id-exchange";

//Time in seconds
pub const SEND_SESSION_DELAY: u64 = 2;
pub const SEND_SESSION_TIMEOUT: u64 = 60;
