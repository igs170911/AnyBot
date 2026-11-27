use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};

use test_harness::FakeClock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{JoinHandle, JoinSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exchange {
    pub(crate) request: String,
    pub(crate) status: Option<u16>,
    pub(crate) key: Option<String>,
    pub(crate) at_ms: Option<u64>,
}

pub(crate) struct Tap {
    port: u16,
    seen: Arc<Mutex<Vec<Exchange>>>,
    beats: Arc<Mutex<Vec<u64>>>,
    lines: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}

#[derive(Clone)]
struct Log {
    seen: Arc<Mutex<Vec<Exchange>>>,
    beats: Arc<Mutex<Vec<u64>>>,
    lines: Arc<Mutex<Vec<String>>>,
    clock: Option<Arc<FakeClock>>,
}

impl Tap {
    pub(crate) async fn start(upstream: SocketAddr) -> Result<Tap, String> {
        Tap::start_timed(upstream, None).await
    }

    pub(crate) async fn start_timed(
        upstream: SocketAddr,
        clock: Option<Arc<FakeClock>>,
    ) -> Result<Tap, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let log = Log {
            seen: Arc::new(Mutex::new(Vec::new())),
            beats: Arc::new(Mutex::new(Vec::new())),
            lines: Arc::new(Mutex::new(Vec::new())),
            clock,
        };
        let seen = Arc::clone(&log.seen);
        let beats = Arc::clone(&log.beats);
        let lines = Arc::clone(&log.lines);
        let task = tokio::spawn(async move {
            let mut live = JoinSet::new();
            while let Ok((client, _from)) = listener.accept().await {
                live.spawn(relay(client, upstream, log.clone()));
            }
        });
        Ok(Tap {
            port,
            seen,
            beats,
            lines,
            task,
        })
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

    pub(crate) fn every_request(&self) -> Vec<String> {
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn beats(&self) -> Vec<u64> {
        self.beats
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

async fn relay(mut client: TcpStream, upstream: SocketAddr, log: Log) {
    let mut head = Vec::new();
    let mut chunk = [0u8; 4096];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        match client.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(count) => head.extend_from_slice(chunk.get(..count).unwrap_or_default()),
        }
    }
    let text = String::from_utf8_lossy(&head).into_owned();
    let line = text.lines().next().unwrap_or_default().to_owned();
    let key = text.lines().find_map(|header| {
        let (name, value) = header.split_once(':')?;
        name.eq_ignore_ascii_case("idempotency-key")
            .then(|| value.trim().to_owned())
    });
    let at_ms = log.clock.as_ref().map(|clock| clock.now_ms());
    note_requests(&log, &head);
    let index = {
        let mut seen = log.seen.lock().unwrap_or_else(PoisonError::into_inner);
        seen.push(Exchange {
            request: line,
            status: None,
            key,
            at_ms,
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
    let forward = forward(&mut client_read, &mut far_write, &log);
    let back = answer(&mut far_read, &mut client_write, &log, index);
    tokio::select! {
        _done = forward => {}
        _done = back => {}
    }
}

async fn forward<R, W>(client: &mut R, far: &mut W, log: &Log)
where
    R: AsyncReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut chunk = [0u8; 16384];
    loop {
        let Ok(count) = client.read(&mut chunk).await else {
            return;
        };
        if count == 0 {
            return;
        }
        let piece = chunk.get(..count).unwrap_or_default();
        note_requests(log, piece);
        if far.write_all(piece).await.is_err() {
            return;
        }
    }
}

fn note_requests(log: &Log, piece: &[u8]) {
    let text = String::from_utf8_lossy(piece);
    let mut lines = log.lines.lock().unwrap_or_else(PoisonError::into_inner);
    for line in text.lines() {
        if line.starts_with("GET /") || line.starts_with("POST /") {
            lines.push(line.to_owned());
        }
    }
}

async fn answer<R, W>(far: &mut R, client: &mut W, log: &Log, index: usize)
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
            note_status(&log.seen, index, piece);
        }
        note_beat(log, piece);
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

fn note_beat(log: &Log, piece: &[u8]) {
    let Some(clock) = log.clock.as_ref() else {
        return;
    };
    if piece.windows(3).any(|window| window == b":hb") {
        let mut beats = log.beats.lock().unwrap_or_else(PoisonError::into_inner);
        beats.push(clock.now_ms());
    }
}
