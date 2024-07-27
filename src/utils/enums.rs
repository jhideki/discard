use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
pub enum SessionType {
    Idle,
    Chat,
    Video,
    Call,
}

#[derive(PartialEq, Clone)]
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
