use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use anybot_foundation::Secret;
use bytes::Bytes;

use crate::transport::TransportFail;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WireMethod {
    Get,
    Post,
}

pub(crate) struct WireHeader {
    pub name: String,
    pub value: Secret,
}

pub(crate) struct WireRequest {
    pub method: WireMethod,
    pub url: String,
    pub headers: Vec<WireHeader>,
    pub body: String,
    pub dial: Option<SocketAddr>,
    pub header_wait: Option<std::time::Duration>,
}

pub(crate) trait HttpWire: Send + Sync {
    fn open<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireOpen, TransportFail>> + Send + 'a>>;

    fn connect<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Connected, TransportFail>> + Send + 'a>> {
        let _kept = request.url.len();
        Box::pin(async { Ok(Connected::Ready) })
    }
}

pub(crate) enum Connected {
    Ready,
}

pub(crate) struct WireOpen {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: WireBody,
}

pub(crate) enum WireBody {
    Stream(Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, TransportFail>> + Send>>),
}

pub(crate) type SharedWire = Arc<dyn HttpWire>;
