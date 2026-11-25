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
    unknown_shape_violations(&content)
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

fn is_header(line: &str, header: &str) -> bool {
    line.trim() == header
}

fn collect_section(content: &str, header: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in content.lines() {
        if is_header(line, header) {
            inside = true;
            continue;
        }
        if line.trim().starts_with('[') {
            inside = false;
            continue;
        }
        if inside && !line.trim().is_empty() {
            lines.push(line.trim().to_owned());
        }
    }
    lines
}

fn unknown_shape_violations(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    if content.contains("[[profile.default.serial]]") {
        out.push(format!(
            "{NEXTEST_CONFIG_REL}: [[profile.default.serial]] is not a nextest key; serial work must use [test-groups] (AGENTS section 3)"
        ));
    }
    if !collect_section(content, "[profile.default.test-groups]").is_empty() {
        out.push(format!(
            "{NEXTEST_CONFIG_REL}: [profile.default.test-groups] is not a nextest key; groups belong in top-level [test-groups] (AGENTS section 3)"
        ));
    }
    for line in collect_section(content, "[[profile.default.overrides]]") {
        let Some((key, _rest)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "group" {
            out.push(format!(
                "{NEXTEST_CONFIG_REL}: overrides use \"group\" which is not a nextest key; assignment is test-group (AGENTS section 3)"
            ));
        }
    }
    out
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
    fn test_nextest_config_gate_blocks_serial_key() {
        let repo = scratch_repo("iron-nextest-serial");
        std::fs::write(
            repo.join(".config").join("nextest.toml"),
            "[profile.default]\nslow-timeout = { period = \"30s\", terminate-after = 4 }\n\n[[profile.default.serial]]\nparallelism = 1\n",
        )
        .unwrap();
        let v = violations(&repo);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|x| x.contains("serial")), "{v:?}");
    }

    #[test]
    fn test_nextest_config_gate_blocks_group_key_in_overrides() {
        let repo = scratch_repo("iron-nextest-group-key");
        std::fs::write(
            repo.join(".config").join("nextest.toml"),
            "[profile.default]\nslow-timeout = { period = \"30s\", terminate-after = 4 }\n\n[[profile.default.overrides]]\nfilter = 'test(x)'\ngroup = \"pg\"\n",
        )
        .unwrap();
        let v = violations(&repo);
        assert_eq!(v.len(), 1);
        assert!(v.first().is_some_and(|x| x.contains("test-group")), "{v:?}");
    }

    #[test]
    fn test_nextest_config_gate_blocks_profile_test_groups_table() {
        let repo = scratch_repo("iron-nextest-profile-groups");
        std::fs::write(
            repo.join(".config").join("nextest.toml"),
            "[profile.default]\nslow-timeout = { period = \"30s\", terminate-after = 4 }\n[profile.default.test-groups]\nparallelism = 1\n",
        )
        .unwrap();
        let v = violations(&repo);
        assert_eq!(v.len(), 1);
        assert!(
            v.first().is_some_and(|x| x.contains("[test-groups]")),
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
