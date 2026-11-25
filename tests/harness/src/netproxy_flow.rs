use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Notify;

use crate::netctl::NetMode;

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
    mode: &NetMode,
    kill: &Arc<Notify>,
) -> Result<usize, String> {
    match mode {
        NetMode::Cut | NetMode::Down | NetMode::HangConnect => Err("cut mid stream".to_owned()),
        NetMode::Buffer => buffer_until_close(client, upstream, kill).await,
        NetMode::Slow {
            first_byte,
            byte_gap,
            ..
        } => pace(client, upstream, *first_byte, *byte_gap, kill).await,
        NetMode::IdleCut { idle } => idle_copy(client, upstream, *idle, kill).await,
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
    }
    let total = collected.len();
    let _sent = client.write_all(&collected).await.is_ok();
    Ok(total)
}

pub(crate) async fn idle_copy(
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
