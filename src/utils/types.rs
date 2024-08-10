use chrono::{DateTime, Utc};
use iroh::net::key::PublicKey;
use std::boxed::Box;
use std::future::Future;
use std::pin::Pin;
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
pub type NodeId = PublicKey; // Alias for public key

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TextMessage {
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl std::fmt::Display for TextMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TextMessage: {} {}", self.content, self.timestamp)
    }
}
