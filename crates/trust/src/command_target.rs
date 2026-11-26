use crate::sandbox_deny::split_segments;

const DELETE_TOOLS: [&str; 6] = ["rm", "rmdir", "unlink", "shred", "trash", "find"];

#[must_use]
pub fn command_target_paths(line: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for segment in split_segments(line) {
        let Some(path) = segment_target(&segment) else {
            continue;
        };
        targets.push(path);
    }
    targets
}

fn segment_target(segment: &str) -> Option<String> {
    let mut parts = segment.split_whitespace();
    let head = parts.next()?;
    if !DELETE_TOOLS.contains(&head) {
        return None;
    }
    parts
        .find(|token| !token.starts_with('-') && !token.contains('='))
        .map(ToOwned::to_owned)
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
}
