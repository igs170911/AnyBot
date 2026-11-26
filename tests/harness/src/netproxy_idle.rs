use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Notify;

use crate::fake_clock::FakeClock;

pub(crate) async fn idle_copy(
    client: &mut TcpStream,
    upstream: &mut TcpStream,
    idle: Duration,
    clock: Option<&Arc<FakeClock>>,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    if let Some(clock) = clock {
        return idle_copy_on_clock(client, upstream, idle, clock, kill).await;
    }
    idle_copy_real(client, upstream, idle, kill).await
}

async fn idle_copy_on_clock(
    client: &mut TcpStream,
    upstream: &mut TcpStream,
    idle: Duration,
    clock: &Arc<FakeClock>,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    let mut chunk = [0u8; 4096];
    let mut total = 0usize;
    loop {
        let mut sleeper = fake_idle_sleep(clock, idle);
        let activity = tokio::select! {
            () = kill.notified() => 0usize,
            read = upstream.read(&mut chunk) => match read {
                Ok(0) => return Ok(total),
                Ok(count) => count,
                Err(_e) => return Ok(total),
            },
            () = sleeper.as_mut() => {
                let _sent = client
                    .write_all(b"HTTP/1.1 502 Idle Timeout\r\n\r\n")
                    .await
                    .is_ok();
                return Ok(total);
            }
        };
        if activity == 0 {
            return Ok(total);
        }
        let Some(piece) = chunk.get(..activity) else {
            return Ok(total);
        };
        let _sent = client.write_all(piece).await.is_ok();
        total = total.saturating_add(activity);
    }
}

async fn idle_copy_real(
    client: &mut TcpStream,
    upstream: &mut TcpStream,
    idle: Duration,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    let mut chunk = [0u8; 4096];
    let mut total = 0usize;
    loop {
        let activity = tokio::select! {
            () = kill.notified() => 0usize,
            read = upstream.read(&mut chunk) => match read {
                Ok(0) => return Ok(total),
                Ok(count) => count,
                Err(_e) => return Ok(total),
            },
            () = tokio::time::sleep(idle) => {
                let _sent = client
                    .write_all(b"HTTP/1.1 502 Idle Timeout\r\n\r\n")
                    .await
                    .is_ok();
                return Ok(total);
            }
        };
        if activity == 0 {
            return Ok(total);
        }
        let Some(piece) = chunk.get(..activity) else {
            return Ok(total);
        };
        let _sent = client.write_all(piece).await.is_ok();
        total = total.saturating_add(activity);
    }
}

fn fake_idle_sleep(
    clock: &Arc<FakeClock>,
    idle: Duration,
) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    let idle_ms = u64::try_from(idle.as_millis()).unwrap_or(u64::MAX);
    let deadline = clock.now_ms().saturating_add(idle_ms);
    clock.wake_at(deadline)
}
