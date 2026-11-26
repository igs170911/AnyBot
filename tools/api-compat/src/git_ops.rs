use std::path::Path;
use std::process::Command;

#[must_use = "the real git reads feed the gate"]
pub fn read_ref(repo: &Path, git_ref: &str, rel: &str) -> Result<Option<String>, String> {
    if !exists_at_ref(repo, git_ref, rel)? {
        return Ok(None);
    }
    let mut spec = git_ref.to_owned();
    spec.push(':');
    spec.push_str(rel);
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["show", &spec])
        .output()
        .map_err(|e| format!("git show {git_ref}:{rel} failed to run: {e}"))?;
    if out.status.success() {
        return Ok(Some(String::from_utf8_lossy(&out.stdout).into_owned()));
    }
    Err(format!(
        "git show {git_ref}:{rel} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    ))
}

fn exists_at_ref(repo: &Path, git_ref: &str, rel: &str) -> Result<bool, String> {
    let mut spec = git_ref.to_owned();
    spec.push(':');
    spec.push_str(rel);
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["cat-file", "-e", &spec])
        .output()
        .map_err(|e| format!("git cat-file failed to run: {e}"))?;
    if out.status.success() {
        return Ok(true);
    }
    if ref_unreadable(repo, git_ref)? {
        return Err(format!(
            "the comparison ref {git_ref} cannot be read; the gate needs a real merge base"
        ));
    }
    Ok(false)
}

fn ref_unreadable(repo: &Path, git_ref: &str) -> Result<bool, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", git_ref])
        .output()
        .map_err(|e| format!("git rev-parse failed to run: {e}"))?;
    Ok(!out.status.success())
}

#[must_use = "the tracked record file is what every clone can check against"]
pub fn changelog_tracked(repo: &Path, rel: &str) -> Result<(), String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["ls-files", "--error-unmatch", rel])
        .output()
        .map_err(|e| format!("git ls-files failed to run: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "{rel} is not tracked by git: the record file must live in the repo (spec/09 K118 6)"
    ))
}

#[must_use = "a released version switches the gate to the post-release rules"]
pub fn release_tags(repo: &Path) -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["tag", "--list"])
        .output()
        .map_err(|e| format!("git tag --list failed to run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git tag --list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    Ok(version_tags(&text))
}

fn version_tags(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let tag = line.trim();
        if tag.starts_with('v') && !tag.to_lowercase().contains("-rc") {
            out.push(tag.to_owned());
        }
    }
    out.sort();
    out
}
