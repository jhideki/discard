use crate::utils::types::TextMessage;
use serde::{Deserialize, Serialize};
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
#[derive(Deserialize, Serialize, Debug)]
pub enum SessionType {
    Idle,
    Chat,
    Video,
    Call,
}

#[derive(PartialEq, Clone, Debug)]
pub enum ConnType {
    Offerer,
    Answerer,
}

#[derive(Deserialize, Serialize)]
pub enum SignalMessageType {
    AddIceCandidate,
    OfferCreated,
    AnswerCreated,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MessageType {
    Message(TextMessage),
    ConnectionState(RTCPeerConnectionState),
}
