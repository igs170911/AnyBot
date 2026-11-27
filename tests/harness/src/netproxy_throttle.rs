use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Notify;

use crate::fake_clock::FakeClock;
use crate::netproxy_flow::{kill_select, read_or_kill};

const CHUNK: usize = 16 * 1024;

pub(crate) async fn throttle_copy(
    client: &mut TcpStream,
    upstream: &mut TcpStream,
    bytes_per_sec: u32,
    clock: Option<&Arc<FakeClock>>,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    let Some(clock) = clock else {
        return Err("throttle mode needs a shared fake clock".to_owned());
    };
    if bytes_per_sec == 0 {
        return Err("throttle rate must be above zero".to_owned());
    }
    let (mut client_read, mut client_write) = client.split();
    let (mut upstream_read, mut upstream_write) = upstream.split();
    let forward = tokio::io::copy(&mut client_read, &mut upstream_write);
    let paced = paced_downstream(
        &mut upstream_read,
        &mut client_write,
        u64::from(bytes_per_sec),
        clock,
        kill,
    );
    tokio::select! {
        sent = paced => sent,
        _forwarded = forward => Ok(0),
    }
}

async fn paced_downstream<R, W>(
    reader: &mut R,
    writer: &mut W,
    bytes_per_sec: u64,
    clock: &Arc<FakeClock>,
    kill: &Arc<Notify>,
) -> Result<usize, String>
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let start_ms = clock.now_ms();
    let mut sent: u64 = 0;
    let mut chunk = vec![0u8; CHUNK];
    loop {
        let Ok(count) = read_or_kill(reader, &mut chunk, kill).await else {
            return finished(sent);
        };
        if count == 0 {
            return finished(sent);
        }
        let mut offset = 0usize;
        while offset < count {
            let allowed = allowance(clock, start_ms, bytes_per_sec).saturating_sub(sent);
            if allowed == 0 {
                let deadline = release_ms(start_ms, bytes_per_sec, sent);
                let waited = kill_select(clock.wake_at(deadline), kill).await;
                if waited.is_err() {
                    return finished(sent);
                }
                continue;
            }
            let left = count.saturating_sub(offset);
            let take = usize::try_from(allowed).map_or(left, |a| a.min(left));
            let end = offset.saturating_add(take);
            let Some(piece) = chunk.get(offset..end) else {
                return finished(sent);
            };
            if writer.write_all(piece).await.is_err() || writer.flush().await.is_err() {
                return finished(sent);
            }
            sent = sent.saturating_add(take as u64);
            offset = end;
        }
    }
}

fn allowance(clock: &Arc<FakeClock>, start_ms: u64, bytes_per_sec: u64) -> u64 {
    let elapsed = clock.now_ms().saturating_sub(start_ms);
    elapsed.saturating_mul(bytes_per_sec) / 1000
}

fn release_ms(start_ms: u64, bytes_per_sec: u64, sent: u64) -> u64 {
    let needed = sent.saturating_add(1).saturating_mul(1000);
    let after = needed.div_ceil(bytes_per_sec);
    start_ms.saturating_add(after)
}

fn finished(sent: u64) -> Result<usize, String> {
    usize::try_from(sent).map_err(|_e| "sent bytes exceed usize".to_owned())
}
