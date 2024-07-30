use iroh::net::key::PublicKey;
use std::boxed::Box;
use std::future::Future;
use std::pin::Pin;
pub type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
pub type NodeId = PublicKey; // Alias for public key
