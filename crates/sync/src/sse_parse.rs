#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub name: Option<String>,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseItem {
    Event(SseEvent),
    Comment,
}

#[derive(Debug, Default)]
pub struct SseParser {
    pending: Vec<u8>,
}

impl SseParser {
    #[must_use]
    pub fn new() -> Self {
        SseParser::default()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<SseItem> {
        self.pending.extend_from_slice(bytes);
        let mut items = Vec::new();
        while let Some((end, gap)) = block_end(&self.pending) {
            let block: Vec<u8> = self.pending.drain(..end.saturating_add(gap)).collect();
            let body = block.get(..end).unwrap_or_default();
            items.push(parse_block(body));
        }
        items
    }
}

fn block_end(buf: &[u8]) -> Option<(usize, usize)> {
    let lf = buf.windows(2).position(|w| w == b"\n\n").map(|at| (at, 2));
    let crlf = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|at| (at, 4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (a, b) => a.or(b),
    }
}

fn parse_block(block: &[u8]) -> SseItem {
    let text = String::from_utf8_lossy(block);
    let mut name: Option<String> = None;
    let mut data: Vec<&str> = Vec::new();
    let mut saw_field = false;
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.starts_with(':') || line.is_empty() {
            continue;
        }
        saw_field = true;
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => name = Some(value.to_owned()),
            "data" => data.push(value),
            _other => {}
        }
    }
    if !saw_field {
        return SseItem::Comment;
    }
    SseItem::Event(SseEvent {
        name,
        data: data.join("\n"),
    })
}

#[cfg(test)]
#[path = "sse_parse_tests.rs"]
mod tests;
