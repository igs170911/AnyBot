pub const MAX_READ_BYTES: u64 = 8_000_000;

use crate::message_codes::{
    CODE_TOOL_READ_DIRECTORY, CODE_TOOL_READ_LINES, CODE_TOOL_READ_NOT_TEXT, CODE_TOOL_READ_TOO_BIG,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadBlock {
    TooBig { size: u64 },
    NotText,
    Directory,
}

impl ReadBlock {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            ReadBlock::TooBig { .. } => CODE_TOOL_READ_TOO_BIG,
            ReadBlock::NotText => CODE_TOOL_READ_NOT_TEXT,
            ReadBlock::Directory => CODE_TOOL_READ_DIRECTORY,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberedLines {
    pub from: usize,
    pub to: usize,
    pub lines: Vec<String>,
}

#[must_use]
pub fn read_code() -> &'static str {
    CODE_TOOL_READ_LINES
}

#[must_use]
pub fn block_of(size: u64, is_directory: bool, content: &str) -> Option<ReadBlock> {
    if size > MAX_READ_BYTES {
        return Some(ReadBlock::TooBig { size });
    }
    if is_directory {
        return Some(ReadBlock::Directory);
    }
    if content.contains('\u{0}') {
        return Some(ReadBlock::NotText);
    }
    None
}

#[must_use]
pub fn numbered_lines(content: &str, offset: usize, limit: usize) -> NumberedLines {
    let collected: Vec<String> = content
        .lines()
        .skip(offset)
        .take(limit)
        .map(|line| (*line).to_owned())
        .collect();
    let from = offset.saturating_add(1);
    let to = from.saturating_add(collected.len());
    NumberedLines {
        from,
        to,
        lines: collected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_numbers_lines_from_the_requested_offset() {
        let out = numbered_lines("a\nb\nc\nd", 1, 2);
        assert_eq!(out.from, 2);
        assert_eq!(out.to, 4);
        assert_eq!(out.lines, vec!["b".to_owned(), "c".to_owned()]);
    }

    #[test]
    fn test_read_blocks_are_reported_for_size_directory_and_binary() {
        assert_eq!(
            block_of(MAX_READ_BYTES.saturating_add(1), false, "text"),
            Some(ReadBlock::TooBig {
                size: MAX_READ_BYTES.saturating_add(1),
            })
        );
        assert_eq!(block_of(10, true, ""), Some(ReadBlock::Directory));
        assert_eq!(block_of(10, false, "a\u{0}b"), Some(ReadBlock::NotText));
        assert_eq!(block_of(10, false, "plain"), None);
    }

    #[test]
    fn test_read_limit_takes_the_requested_window() {
        let out = numbered_lines("a\nb\nc", 0, 2);
        assert_eq!(out.lines.len(), 2);
        assert_eq!(out.to, 3);
    }
}
