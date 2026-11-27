use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{JoinHandle, JoinSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exchange {
    pub(crate) request: String,
    pub(crate) status: Option<u16>,
}

pub(crate) struct Tap {
    port: u16,
    seen: Arc<Mutex<Vec<Exchange>>>,
    task: JoinHandle<()>,
}

impl Tap {
    pub(crate) async fn start(upstream: SocketAddr) -> Result<Tap, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&seen);
        let task = tokio::spawn(async move {
            let mut live = JoinSet::new();
            while let Ok((client, _from)) = listener.accept().await {
                live.spawn(relay(client, upstream, Arc::clone(&log)));
            }
        });
        Ok(Tap { port, seen, task })
    }

    pub(crate) fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub(crate) fn seen(&self) -> Vec<Exchange> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn relay(mut client: TcpStream, upstream: SocketAddr, log: Arc<Mutex<Vec<Exchange>>>) {
    let mut head = Vec::new();
    let mut chunk = [0u8; 4096];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        match client.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(count) => head.extend_from_slice(chunk.get(..count).unwrap_or_default()),
        }
    }
    let line = String::from_utf8_lossy(&head)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    let index = {
        let mut seen = log.lock().unwrap_or_else(PoisonError::into_inner);
        seen.push(Exchange {
            request: line,
            status: None,
        });
        seen.len().saturating_sub(1)
    };
    let Ok(mut far) = TcpStream::connect(upstream).await else {
        return;
    };
    if far.write_all(&head).await.is_err() {
        return;
    }
    let (mut client_read, mut client_write) = client.split();
    let (mut far_read, mut far_write) = far.split();
    let forward = tokio::io::copy(&mut client_read, &mut far_write);
    let back = answer(&mut far_read, &mut client_write, &log, index);
    tokio::select! {
        _done = forward => {}
        _done = back => {}
    }
}

async fn answer<R, W>(far: &mut R, client: &mut W, log: &Arc<Mutex<Vec<Exchange>>>, index: usize)
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut chunk = [0u8; 16384];
    let mut first = true;
    loop {
        let Ok(count) = far.read(&mut chunk).await else {
            return;
        };
        if count == 0 {
            return;
        }
        let piece = chunk.get(..count).unwrap_or_default();
        if first {
            first = false;
            note_status(log, index, piece);
        }
        if client.write_all(piece).await.is_err() {
            return;
        }
    }
}

fn note_status(log: &Arc<Mutex<Vec<Exchange>>>, index: usize, piece: &[u8]) {
    let text = String::from_utf8_lossy(piece);
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok());
    let mut seen = log.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(entry) = seen.get_mut(index) {
        entry.status = status;
    }
}
