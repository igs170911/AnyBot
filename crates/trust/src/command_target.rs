use crate::sandbox_peel::bare_name;
use crate::sandbox_peel::peel_segment;
use crate::sandbox_peel::visible_segments;

const DELETE_TOOLS: [&str; 6] = ["rm", "rmdir", "unlink", "shred", "trash", "find"];

const EXPANSION_TOKENS: [char; 7] = ['$', '~', '*', '?', '[', ']', '`'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteBase {
    Workdir,
    Cd,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteTarget {
    pub token: String,
    pub base: DeleteBase,
    pub cd_path: String,
}

#[must_use]
pub fn command_targets(line: &str) -> Vec<DeleteTarget> {
    let mut out = Vec::new();
    let mut cwd = String::new();
    let mut cwd_known = true;
    for segment in visible_segments(line) {
        let inner = peel_segment(&segment, true);
        let head = inner.split_whitespace().next().unwrap_or_default();
        if head == "cd" {
            let (next, known) = cd_step(&inner, &cwd, cwd_known);
            cwd = next;
            cwd_known = known;
            continue;
        }
        if !DELETE_TOOLS.contains(&bare_name(head).as_str()) {
            continue;
        }
        for token in inner.split_whitespace().skip(1) {
            if token.starts_with('-') || token.contains('=') {
                continue;
            }
            out.push(DeleteTarget {
                token: token.to_owned(),
                base: target_base(cwd_known, &cwd),
                cd_path: cwd.clone(),
            });
        }
    }
    out
}

fn target_base(cwd_known: bool, cwd: &str) -> DeleteBase {
    if !cwd_known {
        return DeleteBase::Unresolved;
    }
    if cwd.is_empty() {
        return DeleteBase::Workdir;
    }
    DeleteBase::Cd
}

fn cd_step(inner: &str, cwd: &str, known: bool) -> (String, bool) {
    let Some(token) = inner
        .split_whitespace()
        .skip(1)
        .find(|t| !t.starts_with('-') && !t.contains('='))
    else {
        return (cwd.to_owned(), known);
    };
    if has_expansion_token(token) || !known {
        return (cwd.to_owned(), false);
    }
    if token.starts_with('/') {
        return (token.to_owned(), true);
    }
    let joined = if cwd.is_empty() {
        token.to_owned()
    } else {
        format!("{cwd}/{token}")
    };
    (joined, true)
}

#[must_use]
pub fn has_expansion_token(token: &str) -> bool {
    token.contains(EXPANSION_TOKENS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_targets_pick_operands_of_every_delete_segment() {
        let got = command_targets("rm notes.md && rmdir build");
        let tokens: Vec<String> = got.iter().map(|t| t.token.clone()).collect();
        assert_eq!(tokens, vec!["notes.md".to_owned(), "build".to_owned()]);
        assert!(command_targets("ls -la").is_empty());
    }

    #[test]
    fn test_command_targets_peel_wrappers_before_reading_the_head() {
        let got = command_targets("rm a.txt; sudo rm -r /opt/app");
        let tokens: Vec<String> = got.iter().map(|t| t.token.clone()).collect();
        assert_eq!(tokens, vec!["a.txt".to_owned(), "/opt/app".to_owned()]);
    }

    #[test]
    fn test_command_targets_follow_a_cd_that_moves_outside_the_workspace() {
        let got = command_targets("cd /tmp && rm -rf build");
        assert_eq!(got.len(), 1);
        assert_eq!(got.first().unwrap().base, DeleteBase::Cd);
        assert_eq!(got.first().unwrap().cd_path, "/tmp");
    }

    #[test]
    fn test_command_targets_mark_targets_unresolvable_after_an_unreadable_cd() {
        let got = command_targets("cd $HOME && rm notes.md");
        assert_eq!(got.len(), 1);
        assert_eq!(got.first().unwrap().base, DeleteBase::Unresolved);
    }

    #[test]
    fn test_command_targets_flag_tokens_that_expand() {
        assert!(has_expansion_token("$HOME/notes.md"));
        assert!(has_expansion_token("~/notes.md"));
        assert!(has_expansion_token("build/*.o"));
        assert!(has_expansion_token("notes[1].md"));
        assert!(!has_expansion_token("notes.md"));
    }
}
