pub const MAX_READ_BYTES: u64 = 8_388_608;
pub const NOT_TEXT_PROBE_BYTES: usize = 8_000;
pub const DEFAULT_READ_LIMIT: usize = 2_000;

use crate::message_codes::{
    CODE_TOOL_READ_DIRECTORY, CODE_TOOL_READ_LINES, CODE_TOOL_READ_MORE, CODE_TOOL_READ_NOT_TEXT,
    CODE_TOOL_READ_TOO_BIG,
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
    pub remaining: usize,
    pub next_offset: usize,
}

#[must_use]
pub fn read_code() -> &'static str {
    CODE_TOOL_READ_LINES
}

#[must_use]
pub fn read_more_code() -> &'static str {
    CODE_TOOL_READ_MORE
}

#[must_use]
pub fn block_of(size: u64, is_directory: bool, content: &str) -> Option<ReadBlock> {
    if size > MAX_READ_BYTES {
        return Some(ReadBlock::TooBig { size });
    }
    if is_directory {
        return Some(ReadBlock::Directory);
    }
    let probe_len = content.len().min(NOT_TEXT_PROBE_BYTES);
    if content
        .as_bytes()
        .get(..probe_len)
        .is_some_and(|head| head.contains(&0))
    {
        return Some(ReadBlock::NotText);
    }
    None
}

#[must_use]
pub fn numbered_lines(content: &str, offset: usize, limit: usize) -> NumberedLines {
    let total = content.lines().count();
    let collected: Vec<String> = content
        .lines()
        .skip(offset)
        .take(limit)
        .map(|line| (*line).to_owned())
        .collect();
    let from = offset.saturating_add(1);
    let to = from.saturating_add(collected.len());
    let next_offset = offset.saturating_add(collected.len());
    let remaining = total.saturating_sub(next_offset);
    NumberedLines {
        from,
        to,
        lines: collected,
        remaining,
        next_offset,
    }
}

#[must_use]
pub fn format_numbered(numbered: &NumberedLines) -> String {
    let lines: Vec<String> = numbered
        .lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let line_no = numbered.from.saturating_add(idx);
            format!("{line_no}: {line}")
        })
        .collect();
    lines.join("\n")
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
