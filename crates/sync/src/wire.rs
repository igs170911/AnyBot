use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::reconnect_status::RequestGuard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireLimits {
    pub connect: Duration,
    pub wait: Duration,
}

impl WireLimits {
    pub const BACKGROUND: WireLimits = WireLimits {
        connect: Duration::from_secs(5),
        wait: Duration::from_secs(30),
    };
    pub const ACTION: WireLimits = WireLimits {
        connect: Duration::from_secs(5),
        wait: Duration::from_secs(20),
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireRequest {
    pub method: WireMethod,
    pub path: String,
    pub body: Option<String>,
    pub idempotency_key: Option<String>,
    pub limits: WireLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireReply {
    pub status: u16,
    pub body: Vec<u8>,
    pub server_time_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireFailure {
    ConnectTimeout,
    ConnectFailed(String),
    WaitTimeout,
    Network(String),
}

pub type WireFuture<'a> = Pin<Box<dyn Future<Output = Result<WireReply, WireFailure>> + Send + 'a>>;

pub trait Wire: Send + Sync {
    fn call<'a>(&'a self, request: WireRequest, guard: &'a RequestGuard) -> WireFuture<'a>;
}
