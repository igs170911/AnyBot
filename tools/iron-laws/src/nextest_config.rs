use std::path::Path;

use crate::collect::read_file_lossy;

pub const NEXTEST_CONFIG_REL: &str = ".config/nextest.toml";

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let path = repo.join(NEXTEST_CONFIG_REL);
    if !path.is_file() {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: missing; nextest runaway tests must be terminated automatically (AGENTS section 3)"
        )];
    }
    let Ok(content) = read_file_lossy(&path) else {
        return vec![format!("{NEXTEST_CONFIG_REL}: cannot be read")];
    };
    let default_body = section_body(&content, "[profile.default]");
    let Some(body) = default_body else {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: has no [profile.default] section (AGENTS section 3)"
        )];
    };
    if !body.contains("terminate-after") {
        return vec![format!(
            "{NEXTEST_CONFIG_REL}: [profile.default] lacks terminate-after; slow-timeout must terminate runaway tests (AGENTS section 3)"
        )];
    }
    Vec::new()
}

fn section_body(content: &str, header: &str) -> Option<String> {
    let mut body = String::new();
    let mut inside = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if trimmed == header {
                inside = true;
            } else if inside {
                break;
            }
            continue;
        }
        if inside {
            body.push_str(trimmed);
            body.push('\n');
        }
    }
    if body.is_empty() {
        return None;
    }
    Some(body)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn scratch_repo(name: &str) -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/scratch");
        let repo = root.join(name);
        std::fs::create_dir_all(repo.join(".config")).unwrap();
        repo
    }

    #[test]
    fn test_nextest_config_gate_blocks_missing_file() {
        let repo = scratch_repo("iron-nextest-missing");
        std::fs::remove_dir_all(repo.join(".config")).unwrap();
        let v = violations(&repo);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|x| x.contains("missing")), "{v:?}");
    }

    #[test]
    fn test_nextest_config_gate_blocks_missing_terminate_after() {
        let repo = scratch_repo("iron-nextest-no-kill");
        std::fs::write(
            repo.join(".config").join("nextest.toml"),
            "[profile.default]\nslow-timeout = { period = \"30s\" }\n",
        )
        .unwrap();
        let v = violations(&repo);
        assert_eq!(v.len(), 1);
        assert!(
            v.first().is_some_and(|x| x.contains("terminate-after")),
            "{v:?}"
        );
    }

    #[test]
    fn test_nextest_config_gate_accepts_real_config() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let v = violations(&repo);
        assert_eq!(v, Vec::<String>::new(), "{v:?}");
    }
}
