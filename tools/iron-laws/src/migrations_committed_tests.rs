#![cfg(test)]

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::migrations_committed::violations;

fn scratch_repo(name: &str) -> PathBuf {
    let test_name = std::thread::current().name().unwrap_or("test").to_owned();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(name)
        .join(test_name);
    remove_scratch(&repo);
    std::fs::create_dir_all(repo.join("migrations/server")).unwrap();
    git(&repo, &["init", "-b", "main"]);
    std::fs::write(
        repo.join("migrations/server/0001_init.sql"),
        "CREATE TABLE notes (id TEXT PRIMARY KEY);\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("migrations/server/0002_second.sql"),
        "ALTER TABLE notes ADD COLUMN body TEXT;\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("migrations/server/0003_third.sql"),
        "CREATE INDEX notes_body ON notes (body);\n",
    )
    .unwrap();
    commit(&repo, "seed the committed migrations");
    repo
}

fn remove_scratch(repo: &Path) {
    match std::fs::remove_dir_all(repo) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => panic!("cannot clear the scratch repo: {e}"),
    }
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn commit(repo: &Path, text: &str) {
    git(repo, &["add", "-A"]);
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
    );
}

#[test]
fn test_committed_migration_gate_allows_a_new_committed_file() {
    let repo = scratch_repo("iron-migrations-committed-unit-add");
    let ref_text = git_rev(&repo, "HEAD");
    std::fs::write(
        repo.join("migrations/server/0002_add.sql"),
        "ALTER TABLE notes ADD COLUMN body TEXT;\n",
    )
    .unwrap();
    commit(&repo, "add a second migration");
    let found = violations(&repo, &ref_text);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn test_committed_migration_gate_blocks_edit_delete_rename() {
    let repo = scratch_repo("iron-migrations-committed-unit-edr");
    let ref_text = git_rev(&repo, "HEAD");
    std::fs::write(
        repo.join("migrations/server/0001_init.sql"),
        "CREATE TABLE notes (id TEXT PRIMARY KEY, extra TEXT);\n",
    )
    .unwrap();
    std::fs::remove_file(repo.join("migrations/server/0002_second.sql")).unwrap();
    git(
        &repo,
        &[
            "mv",
            "migrations/server/0003_third.sql",
            "migrations/server/0003_moved.sql",
        ],
    );
    commit(&repo, "edit, delete and rename committed migrations");
    let found = violations(&repo, &ref_text);
    assert!(
        found
            .iter()
            .any(|line| line.contains("0001_init.sql") && line.contains('M')),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|line| line.contains("0002_second.sql") && line.contains('D')),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|line| line.contains("0003_third.sql") && line.contains('D')),
        "{found:?}"
    );
}

#[test]
fn test_committed_migration_gate_blocks_type_changes() {
    let repo = scratch_repo("iron-migrations-committed-unit-type");
    git_rev(&repo, "HEAD");
    std::fs::remove_file(repo.join("migrations/server/0001_init.sql")).unwrap();
    std::fs::write(
        repo.join("migrations/server/real.sql"),
        "CREATE TABLE notes (id TEXT PRIMARY KEY);\n",
    )
    .unwrap();
    std::os::unix::fs::symlink("real.sql", repo.join("migrations/server/0001_init.sql")).unwrap();
    commit(&repo, "commit the migration as a symlink");
    let ref_text = git_rev(&repo, "HEAD");
    std::fs::remove_file(repo.join("migrations/server/0001_init.sql")).unwrap();
    std::fs::write(
        repo.join("migrations/server/0001_init.sql"),
        "CREATE TABLE notes (id TEXT PRIMARY KEY);\n",
    )
    .unwrap();
    commit(&repo, "turn the symlink into a regular file");
    let found = violations(&repo, &ref_text);
    assert!(
        found
            .iter()
            .any(|line| line.contains("0001_init.sql") && line.contains('T')),
        "{found:?}"
    );
}

fn git_rev(repo: &Path, spec: &str) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", spec])
        .output()
        .unwrap();
    assert!(out.status.success(), "git rev-parse failed");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}
