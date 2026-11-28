use crate::message_codes::{
    CODE_TOOL_READ_RESULT_CUT, CODE_TOOL_READ_RESULT_HIT, CODE_TOOL_READ_RESULT_MISSING,
    CODE_TOOL_READ_RESULT_NOT_FOUND, CODE_TOOL_READ_RESULT_WINDOW,
};

pub const READ_RESULT_MAX_CHARS: usize = 20_000;
pub const RAW_MAX_BYTES: usize = 20 * 1024 * 1024;
pub const FIND_CONTEXT_CHARS: usize = 200;
pub const FIND_MAX_HITS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindHit {
    pub at: usize,
    pub context: String,
}

#[must_use]
pub fn read_result_window_code() -> &'static str {
    CODE_TOOL_READ_RESULT_WINDOW
}

#[must_use]
pub fn read_result_hit_code() -> &'static str {
    CODE_TOOL_READ_RESULT_HIT
}

#[must_use]
pub fn read_result_not_found_code() -> &'static str {
    CODE_TOOL_READ_RESULT_NOT_FOUND
}

#[must_use]
pub fn read_result_missing_code() -> &'static str {
    CODE_TOOL_READ_RESULT_MISSING
}

#[must_use]
pub fn read_result_cut_code() -> &'static str {
    CODE_TOOL_READ_RESULT_CUT
}

#[must_use]
pub fn find_hits(text: &str, needle: &str) -> Vec<FindHit> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    let mut last_byte = 0usize;
    let mut char_pos = 0usize;
    for (byte_idx, matched) in text.match_indices(needle) {
        if hits.len() >= FIND_MAX_HITS {
            break;
        }
        let delta = text
            .get(last_byte..byte_idx)
            .map_or(0, |slice| slice.chars().count());
        char_pos = char_pos.saturating_add(delta);
        let context = context_around(text, byte_idx, matched.len());
        hits.push(FindHit {
            at: char_pos,
            context,
        });
        let matched_chars = matched.chars().count();
        char_pos = char_pos.saturating_add(matched_chars);
        last_byte = byte_idx.saturating_add(matched.len());
    }
    hits
}

fn context_around(text: &str, byte_idx: usize, needle_bytes: usize) -> String {
    let before = text.get(..byte_idx).unwrap_or("");
    let start_byte = before
        .char_indices()
        .rev()
        .nth(FIND_CONTEXT_CHARS.saturating_sub(1))
        .map_or(0, |(idx, _)| idx);
    let after_start = byte_idx.saturating_add(needle_bytes);
    let after = text.get(after_start..).unwrap_or("");
    let after_len = after
        .char_indices()
        .nth(FIND_CONTEXT_CHARS)
        .map_or(after.len(), |(idx, _)| idx);
    let end_byte = after_start.saturating_add(after_len);
    text.get(start_byte..end_byte).unwrap_or("").to_owned()
}

#[must_use]
pub fn clamp_raw_bytes(bytes: Vec<u8>, max_bytes: usize) -> (Vec<u8>, u64) {
    let original_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if bytes.len() <= max_bytes {
        return (bytes, original_len);
    }
    let Some(slice) = bytes.get(..max_bytes) else {
        return (bytes, original_len);
    };
    let cut = utf8_char_boundary(slice);
    let kept = slice.get(..cut).unwrap_or(slice).to_vec();
    (kept, original_len)
}

fn utf8_char_boundary(slice: &[u8]) -> usize {
    let len = slice.len();
    let start = len.saturating_sub(4);
    let tail = slice.get(start..).unwrap_or(slice);
    for offset in 0..tail.len() {
        let probe = tail.get(offset..).unwrap_or(&[]);
        match std::str::from_utf8(probe) {
            Ok(_) => return len,
            Err(err) if err.error_len().is_none() => {
                return start
                    .saturating_add(offset)
                    .saturating_add(err.valid_up_to());
            }
            Err(_) => {}
        }
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_caps_at_twenty() {
        let text = "hit\n".repeat(21);
        let hits = find_hits(&text, "hit");
        assert_eq!(hits.len(), FIND_MAX_HITS);
        assert_eq!(hits.first().map(|h| h.at), Some(0));
    }

    #[test]
    fn find_hits_collects_two_hundred_char_context() {
        let prefix = "p".repeat(300);
        let mid = "m".repeat(500);
        let suffix = "s".repeat(300);
        let text = format!("{prefix}ERR{mid}ERR{suffix}");
        let hits = find_hits(&text, "ERR");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits.first().map(|h| h.at), Some(300));
        assert_eq!(hits.get(1).map(|h| h.at), Some(803));
        assert_eq!(hits.first().map(|h| h.context.chars().count()), Some(403));
    }

    #[test]
    fn clamp_raw_bytes_cuts_on_utf8_char_boundary() {
        let sample = "\u{4e2d}\u{6587}\u{5b57}".as_bytes().to_vec();
        let (kept, original_len) = clamp_raw_bytes(sample, 5);
        assert_eq!(original_len, 9);
        assert_eq!(kept, "\u{4e2d}".as_bytes());
    }
}
