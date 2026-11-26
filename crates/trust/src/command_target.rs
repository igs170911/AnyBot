const DELETE_TOOLS: [&str; 6] = ["rm", "rmdir", "unlink", "shred", "trash", "find"];

const EXPANSION_TOKENS: [char; 7] = ['$', '~', '*', '?', '[', ']', '`'];

const JOIN_TOKENS: [&str; 6] = [";", "&&", "||", "|", "&", "\n"];

#[must_use]
pub fn command_target_paths(line: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for segment in split_segments_raw(line) {
        for token in segment_operands(&segment) {
            targets.push(token);
        }
    }
    targets
}

fn split_segments_raw(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut raw = Vec::new();
    let mut start = 0usize;
    let mut pos = 0usize;
    while pos < bytes.len() {
        let Some(len) = join_token_len_at(bytes, pos) else {
            pos = pos.saturating_add(1);
            continue;
        };
        raw.push(bytes.get(start..pos).unwrap_or(&[]).to_vec());
        pos = pos.saturating_add(len);
        start = pos;
    }
    raw.push(bytes.get(start..).unwrap_or(&[]).to_vec());
    raw.into_iter()
        .map(|chunk| String::from_utf8_lossy(&chunk).trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn join_token_len_at(bytes: &[u8], pos: usize) -> Option<usize> {
    JOIN_TOKENS
        .iter()
        .filter(|t| starts_with_at(bytes, pos, t.as_bytes()))
        .map(|t| t.len())
        .max()
}

fn starts_with_at(bytes: &[u8], pos: usize, token: &[u8]) -> bool {
    let Some(window) = bytes.get(pos..pos.saturating_add(token.len())) else {
        return false;
    };
    window == token
}

fn segment_operands(segment: &str) -> Vec<String> {
    let mut parts = segment.split_whitespace();
    let Some(head) = parts.next() else {
        return Vec::new();
    };
    if !DELETE_TOOLS.contains(&head.to_ascii_lowercase().as_str()) {
        return Vec::new();
    }
    parts
        .filter(|token| !token.starts_with('-') && !token.contains('='))
        .map(ToOwned::to_owned)
        .collect()
}

#[must_use]
pub fn has_expansion_token(token: &str) -> bool {
    token.contains(EXPANSION_TOKENS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_target_paths_pick_the_first_operand_of_each_delete_segment() {
        let got = command_target_paths("rm notes.md && rmdir build");
        assert_eq!(got, vec!["notes.md".to_owned(), "build".to_owned()]);
        assert_eq!(command_target_paths("ls -la"), Vec::<String>::new());
        assert_eq!(
            command_target_paths("rm -rf ./build"),
            vec!["./build".to_owned()]
        );
    }

    #[test]
    fn test_command_target_paths_take_every_operand_of_a_delete() {
        let got = command_target_paths("rm notes.md /etc/passwd");
        assert_eq!(got, vec!["notes.md".to_owned(), "/etc/passwd".to_owned()]);
        assert_eq!(
            command_target_paths("rm A.TXT b.txt"),
            vec!["A.TXT".to_owned(), "b.txt".to_owned()]
        );
    }

    #[test]
    fn test_command_target_paths_flag_tokens_that_expand() {
        assert!(has_expansion_token("$HOME/notes.md"));
        assert!(has_expansion_token("~/notes.md"));
        assert!(has_expansion_token("build/*.o"));
        assert!(has_expansion_token("notes[1].md"));
        assert!(!has_expansion_token("notes.md"));
    }
}
