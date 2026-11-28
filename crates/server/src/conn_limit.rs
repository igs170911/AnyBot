use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll};

use axum::serve::Listener;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use anybot_domain::source_addr::peer_is_unlimited;

pub const PUBLIC_PEER_CONNECTIONS: usize = 48;

type OpenCounts = Arc<Mutex<HashMap<IpAddr, usize>>>;

#[derive(Clone, Default)]
pub struct ConnLimiter {
    open: OpenCounts,
}

pub struct ConnSlot {
    held: Option<(OpenCounts, IpAddr)>,
}

fn counts(open: &OpenCounts) -> MutexGuard<'_, HashMap<IpAddr, usize>> {
    open.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ConnLimiter {
    #[must_use]
    pub fn new() -> Self {
        ConnLimiter::default()
    }

    #[must_use]
    pub fn try_open(&self, peer: IpAddr) -> Option<ConnSlot> {
        if peer_is_unlimited(peer) {
            return Some(ConnSlot { held: None });
        }
        let key = peer.to_canonical();
        let mut open = counts(&self.open);
        let count = open.entry(key).or_insert(0);
        if *count >= PUBLIC_PEER_CONNECTIONS {
            return None;
        }
        *count = count.saturating_add(1);
        Some(ConnSlot {
            held: Some((Arc::clone(&self.open), key)),
        })
    }

    #[must_use]
    pub fn open_count(&self, peer: IpAddr) -> usize {
        counts(&self.open)
            .get(&peer.to_canonical())
            .copied()
            .unwrap_or(0)
    }
}

impl Drop for ConnSlot {
    fn drop(&mut self) {
        let Some((open, key)) = self.held.take() else {
            return;
        };
        let mut map = counts(&open);
        let left = map.get(&key).map_or(0, |count| count.saturating_sub(1));
        if left == 0 {
            map.remove(&key);
        } else {
            map.insert(key, left);
        }
    }
}

pub struct LimitedListener<L> {
    inner: L,
    limiter: ConnLimiter,
}

impl<L> LimitedListener<L> {
    #[must_use]
    pub fn new(inner: L, limiter: ConnLimiter) -> Self {
        LimitedListener { inner, limiter }
    }
}

impl<L> Listener for LimitedListener<L>
where
    L: Listener<Addr = SocketAddr>,
{
    type Io = LimitedIo<L::Io>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (io, addr) = self.inner.accept().await;
            if let Some(slot) = self.limiter.try_open(addr.ip()) {
                return (LimitedIo { io, _slot: slot }, addr);
            }
            drop(io);
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.inner.local_addr()
    }
}

pub struct LimitedIo<T> {
    io: T,
    _slot: ConnSlot,
}

impl<T: AsyncRead + Unpin> AsyncRead for LimitedIo<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_read(cx, buf)
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for LimitedIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().io).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().io).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.io.is_write_vectored()
    }
}

#[cfg(test)]
#[path = "conn_limit_tests.rs"]
mod tests;
