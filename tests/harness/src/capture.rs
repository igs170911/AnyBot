use std::sync::Arc;
use std::sync::Mutex;

pub struct Capture {
    pub bytes: Mutex<Vec<u8>>,
}

impl Default for Capture {
    fn default() -> Self {
        Self::new()
    }
}

impl Capture {
    #[must_use]
    pub fn new() -> Self {
        Capture {
            bytes: Mutex::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let taken: Vec<u8> = match self.bytes.try_lock() {
            Ok(guard) => guard.clone(),
            Err(_poisoned) => Vec::new(),
        };
        let text: String = String::from_utf8_lossy(&taken).into_owned();
        let lines = text.lines().map(str::to_owned);
        lines.collect()
    }
}

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut sink) = self.bytes.try_lock() {
            sink.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct ArcWriter {
    pub sink: Arc<Capture>,
}

impl std::io::Write for ArcWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut bytes) = self.sink.bytes.lock() {
            bytes.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub async fn poll_contains(
    capture: &Arc<Capture>,
    needle: &str,
    seconds: u64,
) -> Result<usize, String> {
    for _tick in 0..seconds.saturating_mul(10) {
        let hits = capture
            .lines()
            .iter()
            .filter(|l| l.contains(needle))
            .count();
        if hits > 0 {
            return Ok(hits);
        }
        crate::realtime::sleep_real(std::time::Duration::from_millis(100)).await;
    }
    Err(format!("no line containing {needle}"))
}
