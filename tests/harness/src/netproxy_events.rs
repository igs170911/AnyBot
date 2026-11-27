use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Notify;

use crate::netproxy_flow::read_or_kill;
use crate::netproxy_response::ResponseTracker;

pub(crate) async fn drop_events_copy(
    client: &mut TcpStream,
    upstream: &mut TcpStream,
    per_mille: u16,
    rng: &Arc<AtomicU64>,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    let (mut client_read, mut client_write) = client.split();
    let (mut upstream_read, mut upstream_write) = upstream.split();
    let forward = tokio::io::copy(&mut client_read, &mut upstream_write);
    let filtered = filter_response(&mut upstream_read, &mut client_write, per_mille, rng, kill);
    tokio::pin!(filtered);
    tokio::select! {
        done = &mut filtered => done,
        _forwarded = forward => filtered.await,
    }
}

async fn filter_response<R, W>(
    reader: &mut R,
    writer: &mut W,
    per_mille: u16,
    rng: &Arc<AtomicU64>,
    kill: &Arc<Notify>,
) -> Result<usize, String>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut tracker = ResponseTracker::new(per_mille, Arc::clone(rng));
    let mut total = 0usize;
    let mut chunk = vec![0u8; 8192];
    loop {
        let Ok(count) = read_or_kill(reader, &mut chunk, kill).await else {
            return Ok(total);
        };
        let Some(piece) = chunk.get(..count) else {
            return Ok(total);
        };
        let out = if count == 0 {
            tracker.finish()
        } else {
            tracker.feed(piece)
        };
        total = total.saturating_add(out.len());
        send(writer, &out).await?;
        if count == 0 {
            return Ok(total);
        }
    }
}

async fn send<W>(writer: &mut W, bytes: &[u8]) -> Result<(), String>
where
    W: AsyncWriteExt + Unpin,
{
    if bytes.is_empty() {
        return Ok(());
    }
    writer
        .write_all(bytes)
        .await
        .map_err(|e| format!("write to client failed: {e}"))?;
    writer
        .flush()
        .await
        .map_err(|e| format!("flush to client failed: {e}"))
}
