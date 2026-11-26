use anybot_foundation::ContentDigest;

use crate::message_codes::{
    CODE_TOOL_SHELL_FAILED, CODE_TOOL_SHELL_NO_OUTPUT, CODE_TOOL_SHELL_TIMED_OUT,
    CODE_TOOL_SHELL_TRIMMED,
};

pub const OUTPUT_EDGE_CHARS: usize = 6000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trimmed {
    pub head: String,
    pub tail: String,
    pub omitted_chars: usize,
    pub full_digest: ContentDigest,
}

#[must_use]
pub fn trim_output(output: &str) -> Option<Trimmed> {
    let head_len = OUTPUT_EDGE_CHARS;
    let tail_len = OUTPUT_EDGE_CHARS;
    if output.len() <= head_len.saturating_add(tail_len) {
        return None;
    }
    let head: String = output.chars().take(head_len).collect();
    let tail: Vec<char> = output.chars().rev().take(tail_len).collect();
    let tail: String = tail.iter().rev().collect();
    Some(Trimmed {
        head,
        tail,
        omitted_chars: output
            .len()
            .saturating_sub(head_len.saturating_add(tail_len)),
        full_digest: ContentDigest::sha256(output.as_bytes()),
    })
}

#[must_use]
pub fn shell_codes(exit_code: Option<i32>, output: &str) -> Vec<&'static str> {
    let mut codes = Vec::new();
    if output.is_empty() {
        codes.push(CODE_TOOL_SHELL_NO_OUTPUT);
    }
    if exit_code != Some(0) {
        codes.push(CODE_TOOL_SHELL_FAILED);
    }
    codes
}

#[must_use]
pub fn trim_code() -> &'static str {
    CODE_TOOL_SHELL_TRIMMED
}

#[must_use]
pub fn timeout_code() -> &'static str {
    CODE_TOOL_SHELL_TIMED_OUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_shell_output_keeps_the_head_and_the_tail_with_a_digest() {
        let output = "x".repeat(OUTPUT_EDGE_CHARS.saturating_mul(3));
        let trimmed = trim_output(&output).unwrap();
        assert_eq!(trimmed.head.len(), OUTPUT_EDGE_CHARS);
        assert_eq!(trimmed.tail.len(), OUTPUT_EDGE_CHARS);
        assert_eq!(trimmed.omitted_chars, OUTPUT_EDGE_CHARS);
        assert_eq!(
            trimmed.full_digest,
            ContentDigest::sha256(output.as_bytes())
        );
    }

    #[test]
    fn test_short_shell_output_is_not_trimmed() {
        assert_eq!(trim_output("short"), None);
        let exact = "y".repeat(OUTPUT_EDGE_CHARS.saturating_mul(2));
        assert_eq!(trim_output(&exact), None);
    }

    #[test]
    fn test_shell_result_codes_name_empty_output_and_failure() {
        assert_eq!(shell_codes(Some(0), ""), vec![CODE_TOOL_SHELL_NO_OUTPUT]);
        assert_eq!(shell_codes(Some(3), "noise"), vec![CODE_TOOL_SHELL_FAILED]);
        assert_eq!(shell_codes(Some(0), "noise"), Vec::<&str>::new());
    }
}
