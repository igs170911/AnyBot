use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

#[must_use]
pub(crate) fn next_random(state: &AtomicU64) -> u64 {
    let step = state.fetch_add(GAMMA, Ordering::SeqCst).wrapping_add(GAMMA);
    let first = (step ^ (step >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let second = (first ^ (first >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    second ^ (second >> 31)
}

enum ChunkState {
    Size,
    Data(usize),
    DataEnd,
    Trailer,
    Done,
}

pub(crate) struct ChunkDecoder {
    buf: Vec<u8>,
    state: ChunkState,
}

impl ChunkDecoder {
    pub(crate) fn new() -> Self {
        ChunkDecoder {
            buf: Vec::new(),
            state: ChunkState::Size,
        }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.buf.extend_from_slice(bytes);
        let mut payload = Vec::new();
        while self.step(&mut payload) {}
        payload
    }

    pub(crate) fn done(&self) -> bool {
        matches!(self.state, ChunkState::Done)
    }

    pub(crate) fn take_rest(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buf)
    }

    fn step(&mut self, payload: &mut Vec<u8>) -> bool {
        match self.state {
            ChunkState::Size => self.read_size(),
            ChunkState::Data(left) => self.read_data(left, payload),
            ChunkState::DataEnd => self.skip_crlf(ChunkState::Size),
            ChunkState::Trailer => self.skip_crlf(ChunkState::Done),
            ChunkState::Done => false,
        }
    }

    fn read_size(&mut self) -> bool {
        let Some(at) = find(&self.buf, b"\r\n") else {
            return false;
        };
        let line: Vec<u8> = self.buf.drain(..at.saturating_add(2)).collect();
        let text = String::from_utf8_lossy(&line);
        let size = text
            .split(';')
            .next()
            .and_then(|digits| usize::from_str_radix(digits.trim(), 16).ok());
        self.state = match size {
            Some(0) => ChunkState::Trailer,
            Some(size) => ChunkState::Data(size),
            None => ChunkState::Done,
        };
        true
    }

    fn read_data(&mut self, left: usize, payload: &mut Vec<u8>) -> bool {
        if self.buf.is_empty() {
            return false;
        }
        let take = left.min(self.buf.len());
        payload.extend(self.buf.drain(..take));
        let remaining = left.saturating_sub(take);
        self.state = if remaining == 0 {
            ChunkState::DataEnd
        } else {
            ChunkState::Data(remaining)
        };
        true
    }

    fn skip_crlf(&mut self, next: ChunkState) -> bool {
        if self.buf.len() < 2 {
            return false;
        }
        self.buf.drain(..2);
        self.state = next;
        true
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn event_end(buf: &[u8]) -> Option<usize> {
    let lf = find(buf, b"\n\n").map(|at| at.saturating_add(2));
    let crlf = find(buf, b"\r\n\r\n").map(|at| at.saturating_add(4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

pub(crate) struct BodyFilter {
    decoder: Option<ChunkDecoder>,
    events: Vec<u8>,
    per_mille: u16,
    rng: Arc<AtomicU64>,
    closed: bool,
}

impl BodyFilter {
    pub(crate) fn new(chunked: bool, per_mille: u16, rng: Arc<AtomicU64>) -> Self {
        BodyFilter {
            decoder: chunked.then(ChunkDecoder::new),
            events: Vec::new(),
            per_mille,
            rng,
            closed: false,
        }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        let payload = match self.decoder.as_mut() {
            Some(decoder) => {
                let payload = decoder.push(bytes);
                if decoder.done() {
                    self.closed = true;
                }
                payload
            }
            None => bytes.to_vec(),
        };
        self.events.extend_from_slice(&payload);
        let mut out = Vec::new();
        while let Some(end) = event_end(&self.events) {
            let event: Vec<u8> = self.events.drain(..end).collect();
            if self.keeps(&event) {
                out.extend(self.frame(&event));
            }
        }
        if self.closed {
            out.extend_from_slice(b"0\r\n\r\n");
        }
        out
    }

    pub(crate) fn finished(&self) -> bool {
        self.closed
    }

    pub(crate) fn take_rest(&mut self) -> Vec<u8> {
        self.decoder
            .as_mut()
            .map_or_else(Vec::new, ChunkDecoder::take_rest)
    }

    pub(crate) fn finish(&mut self) -> Vec<u8> {
        if self.events.is_empty() || self.closed {
            return Vec::new();
        }
        let rest = std::mem::take(&mut self.events);
        self.frame(&rest)
    }

    fn keeps(&self, event: &[u8]) -> bool {
        if event.first() == Some(&b':') {
            return true;
        }
        let roll = next_random(&self.rng) % 1000;
        roll >= u64::from(self.per_mille)
    }

    fn frame(&self, event: &[u8]) -> Vec<u8> {
        if self.decoder.is_none() {
            return event.to_vec();
        }
        let mut framed = format!("{:x}\r\n", event.len()).into_bytes();
        framed.extend_from_slice(event);
        framed.extend_from_slice(b"\r\n");
        framed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng(seed: u64) -> Arc<AtomicU64> {
        Arc::new(AtomicU64::new(seed))
    }

    #[test]
    fn filter_splits_events_that_straddle_chunks() {
        let mut filter = BodyFilter::new(true, 0, rng(1));
        let mut out = Vec::new();
        out.extend(filter.push(b"5\r\ndata:\r\n"));
        out.extend(filter.push(b"3\r\n a\n\r\n"));
        out.extend(filter.push(b"1\r\n\n\r\n0\r\n\r\n"));
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text, "9\r\ndata: a\n\n\r\n0\r\n\r\n");
    }

    #[test]
    fn filter_never_drops_comments_and_drops_all_at_full_ratio() {
        let mut filter = BodyFilter::new(false, 1000, rng(7));
        let out = filter.push(b"event: a\ndata: 1\n\n:hb\n\nevent: b\ndata: 2\n\n");
        assert_eq!(String::from_utf8(out).unwrap(), ":hb\n\n");
    }

    #[test]
    fn filter_keeps_everything_at_zero_ratio() {
        let mut filter = BodyFilter::new(false, 0, rng(7));
        let out = filter.push(b"event: a\ndata: 1\n\nevent: b\ndata: 2\n\n");
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "event: a\ndata: 1\n\nevent: b\ndata: 2\n\n"
        );
    }

    #[test]
    fn same_seed_gives_same_random_sequence() {
        let a = rng(99);
        let b = rng(99);
        let first: Vec<u64> = (0..5).map(|_n| next_random(&a)).collect();
        let second: Vec<u64> = (0..5).map(|_n| next_random(&b)).collect();
        assert_eq!(first, second);
    }
}
