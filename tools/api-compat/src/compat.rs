use std::path::Path;
use std::process::Command;

use crate::log;
use crate::sha;

#[must_use]
pub fn violations(
    repo: &Path,
    git_ref: &str,
    baseline_of: &dyn Fn(&[u8], &[u8]) -> String,
    breaking_of: &dyn Fn(&str, &str) -> Vec<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    let ref_asyncapi = match git_text(repo, git_ref, sha::ASYNCAPI_REL) {
        Ok(text) => text,
        Err(e) => return vec![e],
    };
    let ref_log_text = git_text(repo, git_ref, sha::CHANGELOG_REL).unwrap_or_default();
    let tree_openapi = match sha::read_bytes(&repo.join(sha::OPENAPI_REL)) {
        Ok(bytes) => bytes,
        Err(e) => return vec![e],
    };
    let tree_asyncapi = match sha::read_bytes(&repo.join(sha::ASYNCAPI_REL)) {
        Ok(bytes) => bytes,
        Err(e) => return vec![e],
    };
    let tree_log_text = match std::fs::read_to_string(repo.join(sha::CHANGELOG_REL)) {
        Ok(text) => text,
        Err(e) => return vec![format!("cannot read {}: {e}", sha::CHANGELOG_REL)],
    };
    out.extend(log::format_violations(sha::CHANGELOG_REL, &tree_log_text));
    push_sha_violations(
        &tree_openapi,
        &tree_asyncapi,
        &tree_log_text,
        baseline_of,
        &mut out,
    );
    push_change_violations(
        &ref_asyncapi,
        &tree_asyncapi,
        &ref_log_text,
        &tree_log_text,
        breaking_of,
        &mut out,
    );
    out
}

fn push_sha_violations(
    tree_openapi: &[u8],
    tree_asyncapi: &[u8],
    tree_log_text: &str,
    baseline_of: &dyn Fn(&[u8], &[u8]) -> String,
    out: &mut Vec<String>,
) {
    let want = baseline_of(tree_openapi, tree_asyncapi);
    let lines = log::entries(tree_log_text);
    let Some(last) = lines.last() else {
        return;
    };
    if !sha::is_hex64(&last.hash) {
        out.push(format!(
            "{}: a changelog line carries a hash that is not 64 hex digits",
            sha::CHANGELOG_REL
        ));
        return;
    }
    if last.hash != want {
        out.push(format!(
            "{}: the last changelog line names sha256={} but the committed baselines hash to {}; a regenerated baseline must record its own hash",
            sha::CHANGELOG_REL, last.hash, want
        ));
    }
}

fn push_change_violations(
    ref_asyncapi: &str,
    tree_asyncapi: &[u8],
    ref_log_text: &str,
    tree_log_text: &str,
    breaking_of: &dyn Fn(&str, &str) -> Vec<String>,
    out: &mut Vec<String>,
) {
    let tree_text = String::from_utf8_lossy(tree_asyncapi).into_owned();
    let breaking = breaking_of(ref_asyncapi, &tree_text);
    for item in &breaking {
        out.push(format!(
            "{}: breaking protocol change: {item}",
            sha::ASYNCAPI_REL
        ));
    }
    let schema_changed = tree_text != ref_asyncapi;
    if !breaking.is_empty() && !schema_changed {
        out.push(format!(
            "{}: the tree protocol breaks the committed protocol; regenerate the committed baselines in the same commit",
            sha::ASYNCAPI_REL
        ));
    }
    let ref_lines = log::entries(ref_log_text).len();
    let tree_lines = log::entries(tree_log_text).len();
    if schema_changed && tree_lines <= ref_lines {
        out.push(format!(
            "{}: the committed baselines changed; api/breaking-changes.log must grow with a dated record of what changed and why (spec/09 K118 6)",
            sha::CHANGELOG_REL
        ));
    }
}

fn ref_spec(git_ref: &str, rel: &str) -> String {
    let mut spec = git_ref.to_owned();
    spec.push(':');
    spec.push_str(rel);
    spec
}

fn git_text(repo: &Path, git_ref: &str, rel: &str) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["show", &ref_spec(git_ref, rel)])
        .output()
        .map_err(|e| format!("git show {git_ref}:{rel} failed to run: {e}"))?;
    if !out.status.success() {
        let mut message = "git show ".to_owned();
        message.push_str(git_ref);
        message.push(':');
        message.push_str(rel);
        message.push_str(" failed: the committed baseline is missing at the ref; commit the baselines first (api-docs gate)");
        return Err(message);
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
