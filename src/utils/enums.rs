use core::fmt;

use crate::utils::types::{NodeId, TextMessage};
use serde::{Deserialize, Serialize};
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
#[derive(Deserialize, Serialize, Debug)]
pub enum SessionType {
    Idle,
    Chat,
    Video,
    Call,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UserStatus {
    Online,
    Away,
    Offline,
}

impl fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self {
            UserStatus::Offline => "offline",
            UserStatus::Online => "online",
            UserStatus::Away => "away",
        };
        write!(f, "{}", status)
    }
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

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum SignalMessage {
    Online,
    SendConnection,
}

//Signals what the client should prepare for. E.g., ReceiveMessage will signal the client to
//prepare to recieve an incoming message.
#[derive(PartialEq, Clone, Debug)]
pub enum RunMessage {
    Online,
    SendMessage((NodeId, TextMessage)),
    ReceiveMessage,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MessageType {
    Message(TextMessage),
    ConnectionState(RTCPeerConnectionState),
}
