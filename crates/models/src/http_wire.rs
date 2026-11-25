use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;

use crate::transport::TransportFail;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WireMethod {
    Get,
    Post,
}

#[derive(Clone, Debug)]
pub(crate) struct WireRequest {
    pub method: WireMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Clone, Debug)]
pub(crate) struct WireResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

pub(crate) trait HttpWire: Send + Sync {
    fn send<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireResponse, TransportFail>> + Send + 'a>>;

    fn open<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireOpen, TransportFail>> + Send + 'a>>;
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
