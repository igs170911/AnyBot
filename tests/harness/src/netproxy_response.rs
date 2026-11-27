use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use crate::netproxy_sse_filter::{BodyFilter, ChunkDecoder};

enum Body {
    Head(Vec<u8>),
    Events(BodyFilter),
    Length(usize),
    Chunked(ChunkDecoder),
    UntilClose,
}

pub(crate) struct ResponseTracker {
    body: Body,
    per_mille: u16,
    rng: Arc<AtomicU64>,
}

struct Head {
    events: bool,
    chunked: bool,
    length: Option<usize>,
    bodyless: bool,
}

impl ResponseTracker {
    pub(crate) fn new(per_mille: u16, rng: Arc<AtomicU64>) -> Self {
        ResponseTracker {
            body: Body::Head(Vec::new()),
            per_mille,
            rng,
        }
    }

    pub(crate) fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut pending = bytes.to_vec();
        while !pending.is_empty() {
            pending = self.step(pending, &mut out);
        }
        out
    }

    pub(crate) fn finish(&mut self) -> Vec<u8> {
        match &mut self.body {
            Body::Events(filter) => filter.finish(),
            Body::Head(seen) => std::mem::take(seen),
            Body::Length(_) | Body::Chunked(_) | Body::UntilClose => Vec::new(),
        }
    }

    fn step(&mut self, pending: Vec<u8>, out: &mut Vec<u8>) -> Vec<u8> {
        match &mut self.body {
            Body::Head(_) => self.take_head(pending, out),
            Body::Events(filter) => {
                out.extend(filter.push(&pending));
                if !filter.finished() {
                    return Vec::new();
                }
                let rest = filter.take_rest();
                self.body = Body::Head(Vec::new());
                rest
            }
            Body::Length(left) => {
                let take = (*left).min(pending.len());
                let (used, rest) = pending.split_at(take);
                out.extend_from_slice(used);
                *left = left.saturating_sub(take);
                let rest = rest.to_vec();
                if *left == 0 {
                    self.body = Body::Head(Vec::new());
                }
                rest
            }
            Body::Chunked(decoder) => {
                let _payload = decoder.push(&pending);
                if !decoder.done() {
                    out.extend_from_slice(&pending);
                    return Vec::new();
                }
                let rest = decoder.take_rest();
                let used = pending.len().saturating_sub(rest.len());
                let (sent, _tail) = pending.split_at(used);
                out.extend_from_slice(sent);
                self.body = Body::Head(Vec::new());
                rest
            }
            Body::UntilClose => {
                out.extend_from_slice(&pending);
                Vec::new()
            }
        }
    }

    fn take_head(&mut self, pending: Vec<u8>, out: &mut Vec<u8>) -> Vec<u8> {
        let Body::Head(seen) = &mut self.body else {
            return pending;
        };
        seen.extend_from_slice(&pending);
        let Some(at) = seen.windows(4).position(|w| w == b"\r\n\r\n") else {
            return Vec::new();
        };
        let end = at.saturating_add(4);
        let rest = seen.split_off(end);
        let head = std::mem::take(seen);
        out.extend_from_slice(&head);
        self.body = self.body_for(&parse_head(&head));
        rest
    }

    fn body_for(&self, head: &Head) -> Body {
        if head.bodyless {
            return Body::Head(Vec::new());
        }
        match (head.events, head.chunked, head.length) {
            (true, chunked, _) => Body::Events(BodyFilter::new(
                chunked,
                self.per_mille,
                Arc::clone(&self.rng),
            )),
            (false, true, _) => Body::Chunked(ChunkDecoder::new()),
            (false, false, Some(0)) => Body::Head(Vec::new()),
            (false, false, Some(length)) => Body::Length(length),
            (false, false, None) => Body::UntilClose,
        }
    }
}

fn parse_head(head: &[u8]) -> Head {
    let text = String::from_utf8_lossy(head).to_ascii_lowercase();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok());
    let length = text.lines().find_map(|line| {
        line.strip_prefix("content-length:")
            .and_then(|v| v.trim().parse::<usize>().ok())
    });
    Head {
        events: text.contains("content-type: text/event-stream"),
        chunked: text.contains("transfer-encoding: chunked"),
        length,
        bodyless: matches!(status, Some(100..=199 | 204 | 304)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_in_pieces(wire: &[u8], size: usize, per_mille: u16) -> Vec<u8> {
        let mut tracker = ResponseTracker::new(per_mille, Arc::new(AtomicU64::new(3)));
        let mut out = Vec::new();
        for piece in wire.chunks(size) {
            out.extend(tracker.feed(piece));
        }
        out.extend(tracker.finish());
        out
    }

    fn two_responses_on_one_connection() -> Vec<u8> {
        let mut wire = Vec::new();
        wire.extend_from_slice(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello");
        wire.extend_from_slice(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n",
        );
        wire.extend_from_slice(b"12\r\nevent: a\ndata: 1\n\n\r\n0\r\n\r\n");
        wire.extend_from_slice(b"HTTP/1.1 204 No Content\r\n\r\n");
        wire.extend_from_slice(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n0\r\n\r\n",
        );
        wire
    }

    #[test]
    fn tracker_passes_everything_through_at_zero_ratio_for_any_split() {
        let wire = two_responses_on_one_connection();
        for size in [1, 2, 3, 7, 64, 4096] {
            assert_eq!(feed_in_pieces(&wire, size, 0), wire, "split size {size}");
        }
    }

    #[test]
    fn tracker_filters_only_the_event_stream_response_and_keeps_framing() {
        let wire = two_responses_on_one_connection();
        let out = feed_in_pieces(&wire, 5, 1_000);
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello"));
        assert!(!text.contains("event: a"));
        assert!(text.contains("\r\n\r\n0\r\n\r\nHTTP/1.1 204 No Content"));
        assert!(text.ends_with("3\r\nabc\r\n0\r\n\r\n"));
    }
}
