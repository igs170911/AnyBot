use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Notify;

use crate::netctl::NetMode;
use crate::netproxy::TargetSpec;
use crate::netproxy_events::drop_events_copy;
use crate::netproxy_idle::idle_copy;
use crate::netproxy_throttle::throttle_copy;

pub(crate) async fn sleep_with_kill(dur: Duration, kill: &Arc<Notify>) {
    let nap = tokio::time::sleep(dur);
    let wake = kill.notified();
    tokio::select! {
        _done = nap => {}
        _wake = wake => {}
    }
}

pub(crate) async fn copy_bidirectional_with_kill(
    client: &mut TcpStream,
    upstream: &mut TcpStream,
    spec: &TargetSpec,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    match &spec.mode {
        NetMode::Cut | NetMode::Down | NetMode::HangConnect => Err("cut mid stream".to_owned()),
        NetMode::Buffer => buffer_until_close(client, upstream, kill).await,
        NetMode::Slow {
            first_byte,
            byte_gap,
            ..
        } => pace(client, upstream, *first_byte, *byte_gap, kill).await,
        NetMode::IdleCut { idle } => {
            idle_copy(client, upstream, *idle, spec.clock.as_ref(), kill).await
        }
        NetMode::Stall => stall_copy(client, upstream, kill).await,
        NetMode::DropResponse { close } => drop_copy(client, upstream, *close, kill).await,
        NetMode::Throttle { bytes_per_sec } => {
            throttle_copy(client, upstream, *bytes_per_sec, spec.clock.as_ref(), kill).await
        }
        NetMode::DropEvents { per_mille, .. } => {
            drop_events_copy(client, upstream, *per_mille, &spec.event_rng, kill).await
        }
        NetMode::Open => {
            let copied = kill_select(tokio::io::copy_bidirectional(client, upstream), kill).await;
            match copied {
                Ok(Ok((up, _down))) => {
                    let Ok(size) = usize::try_from(up) else {
                        return Err("copied bytes exceed usize".to_owned());
                    };
                    Ok(size)
                }
                Ok(Err(_e)) => Err("copy failed".to_owned()),
                Err(()) => Err("copy killed".to_owned()),
            }
        }
    }
}

pub(crate) async fn buffer_until_close(
    client: &mut TcpStream,
    mut upstream: &mut TcpStream,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    let mut collected = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut streamed = false;
    let mut total = 0usize;
    loop {
        let read = read_or_kill(&mut upstream, &mut chunk, kill).await;
        let Ok(count) = read else {
            break;
        };
        if count == 0 {
            break;
        }
        let Some(piece) = chunk.get(..count) else {
            break;
        };
        collected.extend_from_slice(piece);
        total = total.saturating_add(count);
        if !streamed && head_of(&collected).is_some_and(|head| markers_no_buffering(&head)) {
            streamed = true;
            let _sent = client.write_all(&collected).await.is_ok();
            let _flushed = client.flush().await.is_ok();
        } else if streamed {
            let _sent = client.write_all(piece).await.is_ok();
            let _flushed = client.flush().await.is_ok();
        }
    }
    if !streamed {
        let _sent = client.write_all(&collected).await.is_ok();
    }
    Ok(total)
}

fn head_of(collected: &[u8]) -> Option<String> {
    let tail = collected.windows(4).position(|w| w == b"\r\n\r\n")?;
    let end = tail.checked_add(4).unwrap_or(collected.len());
    let head = collected.get(..end)?;
    Some(String::from_utf8_lossy(head).into_owned())
}

fn markers_no_buffering(head: &str) -> bool {
    head.to_ascii_lowercase().contains("x-accel-buffering: no")
}

pub(crate) async fn pace(
    client: &mut TcpStream,
    mut upstream: &mut TcpStream,
    first_byte: Duration,
    byte_gap: Duration,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    sleep_with_kill(first_byte, kill).await;
    let mut chunk = [0u8; 4096];
    let mut total = 0usize;
    loop {
        let read = read_or_kill(&mut upstream, &mut chunk, kill).await;
        let Ok(count) = read else {
            break;
        };
        if count == 0 {
            break;
        }
        let Some(piece) = chunk.get(..count) else {
            break;
        };
        for byte in piece {
            let single = [*byte];
            let _sent = client.write_all(&single).await.is_ok();
            let _flushed = client.flush().await.is_ok();
            total = total.saturating_add(1);
            sleep_with_kill(byte_gap, kill).await;
        }
    }
    Ok(total)
}

pub(crate) async fn read_or_kill<R>(
    reader: &mut R,
    buf: &mut [u8],
    kill: &Arc<Notify>,
) -> Result<usize, ()>
where
    R: AsyncReadExt + Unpin,
{
    tokio::select! {
        () = kill.notified() => Err(()),
        read = reader.read(buf) => read.map_err(|_e| ()),
    }
}

pub(crate) async fn kill_select<F: Future>(future: F, kill: &Arc<Notify>) -> Result<F::Output, ()> {
    tokio::select! {
        () = kill.notified() => Err(()),
        done = future => Ok(done),
    }
}

pub(crate) async fn stall_copy(
    client: &mut TcpStream,
    upstream: &mut TcpStream,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    let (mut client_read, _client_write) = client.split();
    let (mut upstream_read, _upstream_write) = upstream.split();
    let mut client_chunk = [0u8; 4096];
    let mut upstream_chunk = [0u8; 4096];
    loop {
        tokio::select! {
            () = kill.notified() => return Ok(0),
            read = client_read.read(&mut client_chunk) => match read {
                Ok(0) => {
                    let _pending = tokio::time::sleep(Duration::from_secs(3600)).await;
                    return Ok(0);
                }
                Ok(_count) => {}
                Err(_e) => return Ok(0),
            },
            read = upstream_read.read(&mut upstream_chunk) => match read {
                Ok(0) => {
                    let _pending = tokio::time::sleep(Duration::from_secs(3600)).await;
                    return Ok(0);
                }
                Ok(_count) => {}
                Err(_e) => return Ok(0),
            },
        }
    }
}

pub(crate) async fn drop_copy(
    client: &mut TcpStream,
    upstream: &mut TcpStream,
    close: bool,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    let (mut client_read, mut upstream_write) = client.split();
    let (mut upstream_read, _client_write) = upstream.split();
    let forward = tokio::io::copy(&mut client_read, &mut upstream_write);
    let drain = drain_upstream(&mut upstream_read, kill);
    tokio::select! {
        _sent = forward => Ok(0),
        drained = drain => {
            match drained {
                Ok(_bytes) => {}
                Err(e) => return Err(e),
            }
            if !close {
                let _pending = kill.notified().await;
            }
            Ok(0)
        }
    }
}

async fn drain_upstream<R>(reader: &mut R, kill: &Arc<Notify>) -> Result<usize, String>
where
    R: AsyncReadExt + Unpin,
{
    let mut chunk = [0u8; 4096];
    let mut swallowed = 0usize;
    loop {
        let read = read_or_kill(reader, &mut chunk, kill).await;
        let Ok(count) = read else {
            return Ok(swallowed);
        };
        if count == 0 {
            return Ok(swallowed);
        }
        swallowed = swallowed.saturating_add(count);
    }
}
