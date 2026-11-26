use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::compat;
use crate::git_ops;
use crate::schema::breaking;
use crate::sha::baseline;

pub fn scratch_repo(name: &str) -> Result<PathBuf, String> {
    let test_name = std::thread::current().name().unwrap_or("test").to_owned();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(name)
        .join(test_name);
    remove_scratch(&repo)?;
    std::fs::create_dir_all(repo.join("api"))
        .map_err(|e| format!("cannot create the scratch api dir: {e}"))?;
    git(&repo, &["init", "-b", "main"])?;
    std::fs::write(repo.join(".gitignore"), "target/\n")
        .map_err(|e| format!("cannot write the scratch gitignore: {e}"))?;
    commit(&repo, "initial empty commit")?;
    Ok(repo)
}

pub fn remove_scratch(repo: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(repo) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("cannot clear the scratch repo: {e}")),
    }
}

pub fn git(repo: &Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .map_err(|e| format!("git {args:?} failed to run: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {args:?} exited with {status}"))
    }
}

pub fn git_out(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("git {args:?} failed to run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?} exited with a failure: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

pub fn fixture(name: &str) -> Result<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/api-compat-selftest/fixtures")
        .join(name);
    std::fs::read_to_string(path).map_err(|e| format!("cannot read the fixture {name}: {e}"))
}

pub fn commit(repo: &Path, text: &str) -> Result<(), String> {
    git(repo, &["add", "-A"])?;
    git(
        repo,
        &[
            "-c",
            "user.email=selftest@invalid",
            "-c",
            "user.name=selftest",
            "commit",
            "--allow-empty",
            "-m",
            text,
        ],
    )
}

pub fn write_api(repo: &Path, openapi_text: &str, asyncapi_text: &str) -> Result<(), String> {
    std::fs::write(repo.join("api/openapi.yaml"), openapi_text)
        .map_err(|e| format!("cannot write the scratch openapi: {e}"))?;
    std::fs::write(repo.join("api/asyncapi.yaml"), asyncapi_text)
        .map_err(|e| format!("cannot write the scratch asyncapi: {e}"))
}

#[must_use = "the violations decide the gate verdict"]
pub fn run(repo: &Path, git_ref: &str) -> Vec<String> {
    compat::violations(&inputs(repo, git_ref))
}

#[must_use = "the violations and notes decide the gate verdict"]
pub fn run_notes(repo: &Path, git_ref: &str) -> (Vec<String>, Vec<String>) {
    compat::violations_with_notes(&inputs(repo, git_ref))
}

fn inputs<'a>(repo: &'a Path, git_ref: &'a str) -> compat::Inputs<'a> {
    compat::Inputs {
        repo,
        git_ref,
        read_ref: &git_ops::read_ref,
        tracked: &git_ops::changelog_tracked,
        release_tags: &git_ops::release_tags,
        breaking_of: &breaking,
    }
}

pub fn seed(repo: &Path) -> Result<String, String> {
    let openapi = fixture("openapi.yaml")?;
    let asyncapi = fixture("asyncapi-base.yaml")?;
    write_api(repo, &openapi, &asyncapi)?;
    let sha = baseline(openapi.as_bytes(), asyncapi.as_bytes());
    let line = format!(
        "2026-01-01 | sha256={sha} | fixtures seeded | the selftest starts from a committed baseline\n"
    );
    std::fs::write(repo.join("api/breaking-changes.log"), line)
        .map_err(|e| format!("cannot write the seed record: {e}"))?;
    commit(repo, "seed")?;
    Ok(sha)
}

pub fn touch_changelog(repo: &Path) -> Result<(), String> {
    let mut text = read_log(repo)?;
    text.push_str("# a trailing comment line: the files themselves do not change\n");
    std::fs::write(repo.join("api/breaking-changes.log"), text)
        .map_err(|e| format!("cannot touch the scratch record: {e}"))?;
    commit(repo, "touch the changelog comment")
}

pub fn append_record(repo: &Path, sha: &str, what: &str) -> Result<(), String> {
    let mut text = read_log(repo)?;
    let line = format!("2026-01-02 | sha256={sha} | {what} | recorded for the gate test\n");
    text.push_str(&line);
    std::fs::write(repo.join("api/breaking-changes.log"), text)
        .map_err(|e| format!("cannot append the scratch record: {e}"))
}

pub fn write_log(repo: &Path, text: &str) -> Result<(), String> {
    std::fs::write(repo.join("api/breaking-changes.log"), text)
        .map_err(|e| format!("cannot write the scratch record: {e}"))
}

fn read_log(repo: &Path) -> Result<String, String> {
    std::fs::read_to_string(repo.join("api/breaking-changes.log"))
        .map_err(|e| format!("cannot read the scratch record: {e}"))
}

pub fn merge_base(repo: &Path) -> Result<String, String> {
    git_out(repo, &["merge-base", "HEAD", "dev"])
}

pub fn start_branch(repo: &Path) -> Result<(), String> {
    git(repo, &["branch", "dev"])
}

pub fn commit_sha(repo: &Path) -> Result<String, String> {
    git_out(repo, &["rev-parse", "HEAD"])
}

pub fn binary(exe: &Path, repo: &Path, git_ref: &str) -> Result<(String, bool), String> {
    let out = Command::new(exe)
        .args(["--against", git_ref, "--repo", "."])
        .current_dir(repo)
        .output()
        .map_err(|e| format!("the api-compat binary failed to run: {e}"))?;
    Ok((
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.success(),
    ))
}
