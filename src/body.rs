use hyper::body::{Body, Buf, Bytes};
use std::convert::Infallible;
pub struct MessageBody {
    data: Vec<u8>,
    position: usize,
}
impl Body for MessageBody {
    type Data = Buf;
    type Error = Infallible;
    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
    }
}
