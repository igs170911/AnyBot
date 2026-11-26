use crate::command_read::{ReadCommand, ReadLine, read_line};

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
    let ReadLine::Read(commands) = read_line(line) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut cwd = String::new();
    let mut cwd_known = true;
    for command in commands {
        if command.name == "cd" {
            let (next, known) = cd_step(&command, &cwd, cwd_known);
            cwd = next;
            cwd_known = known;
            continue;
        }
        if !DELETE_TOOLS.contains(&command.name.as_str()) {
            continue;
        }
        for token in command.args.iter().filter(|token| !token.starts_with('-')) {
            out.push(DeleteTarget {
                token: token.clone(),
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

fn cd_step(command: &ReadCommand, cwd: &str, known: bool) -> (String, bool) {
    let Some(token) = command.args.iter().find(|token| !token.starts_with('-')) else {
        return (cwd.to_owned(), false);
    };
    let token = token.as_str();
    if token == "-" {
        return (cwd.to_owned(), false);
    }
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
    fn test_command_targets_drop_a_line_whose_cd_is_not_a_plain_path() {
        assert!(command_targets("cd $HOME && rm notes.md").is_empty());
        assert!(command_targets("cd ; rm notes.md").is_empty());
        assert!(command_targets("cd - && rm notes.md").is_empty());
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
