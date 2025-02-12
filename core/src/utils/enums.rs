use core::fmt;
use std::str::FromStr;

use crate::utils::errors::ParseEnumError;
use crate::utils::types::{NodeId, TextMessage};
use serde::{Deserialize, Serialize};
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum SessionType {
    Idle,
    Chat,
    Video,
    Call,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
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

impl FromStr for UserStatus {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "offline" => Ok(UserStatus::Offline),
            "online" => Ok(UserStatus::Online),
            "away" => Ok(UserStatus::Away),
            _ => Err(ParseEnumError::InvalidVariant),
        }
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
    Online(NodeId, UserStatus),
    SendConnection(SessionType),
}

//Signals what the client should prepare for. E.g., ReceiveMessage will signal the client to
//prepare to recieve an incoming message.
#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RunMessage {
    UpdateStatus(NodeId, UserStatus),
    Adduser(NodeId, String),
    InitConn(SessionType, String),
    RecvConn(SessionType),
    GetUsers,
    Shutdown,
    SendMessage(String, String),
    GetUser(String),
}

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum RunMessageType {
    Online,
    SendMessage,
    AddUser,
    ReceiveMessage,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MessageType {
    Message(TextMessage),
    ConnectionState(RTCPeerConnectionState),
}
