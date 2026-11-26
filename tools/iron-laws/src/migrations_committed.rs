use std::path::Path;
use std::process::Command;

use crate::migrations::MIGRATION_DIRS;

#[must_use]
pub fn violations(repo: &Path, git_ref: &str) -> Vec<String> {
    let mut out = Vec::new();
    for rel_dir in MIGRATION_DIRS {
        out.extend(dir_violations(repo, git_ref, rel_dir));
    }
    out
}

fn dir_violations(repo: &Path, git_ref: &str, rel_dir: &str) -> Vec<String> {
    let unreadable = format!(
        "{rel_dir}: comparison ref {git_ref} cannot be read; the migration gate needs a ref with the committed migrations"
    );
    let list = match git_output(
        repo,
        &["ls-tree", "-r", "--name-only", git_ref, "--", rel_dir],
    ) {
        Ok(text) => text,
        Err(_e) => return vec![unreadable],
    };
    let mut out = Vec::new();
    for line in list.lines() {
        if !crate::table::has_extension(line, "sql") {
            continue;
        }
        if !repo.join(line).exists() {
            out.push(format!(
                "{line}: a committed migration is missing locally; committed migrations never change"
            ));
        }
    }
    out.extend(diff_violations(repo, git_ref, rel_dir));
    out
}

fn diff_violations(repo: &Path, git_ref: &str, rel_dir: &str) -> Vec<String> {
    let text = match git_output(
        repo,
        &[
            "diff",
            "--name-status",
            "--no-renames",
            "--diff-filter=DMT",
            git_ref,
            "--",
            rel_dir,
        ],
    ) {
        Ok(text) => text,
        Err(e) => return vec![e],
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.split('\t');
        let (Some(kind), Some(file)) = (parts.next(), parts.next()) else {
            out.push(format!("{rel_dir}: unparsed diff line {line}"));
            continue;
        };
        out.push(format!(
            "{file}: a committed migration was {kind} against {git_ref}; committed migrations never change (spec/09 K118 6)"
        ));
    }
    out
}

fn git_output(repo: &Path, args: &[&str]) -> Result<String, String> {
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
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn scratch_repo(name: &str) -> PathBuf {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/scratch")
            .join(name);
        remove_scratch(&repo);
        std::fs::create_dir_all(repo.join("migrations/server")).unwrap();
        std::fs::create_dir_all(repo.join("migrations/client")).unwrap();
        git(&repo, &["init", "-b", "main"]);
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

    fn commit(repo: &Path) {
        git(repo, &["add", "migrations"]);
        git(
            repo,
            &[
                "-c",
                "user.email=selftest@invalid",
                "-c",
                "user.name=selftest",
                "commit",
                "-m",
                "seed",
            ],
        );
    }

    #[test]
    fn test_committed_migration_gate_blocks_content_edits_of_committed_files() {
        let repo = scratch_repo("iron-migrations-committed-edit");
        let file = repo.join("migrations/server/0001_init.sql");
        std::fs::write(&file, "CREATE TABLE a (id BIGINT);\n").unwrap();
        commit(&repo);
        std::fs::write(&file, "CREATE TABLE a (id BIGINT PRIMARY KEY);\n").unwrap();
        commit(&repo);
        let found = violations(&repo, "HEAD~1");
        assert!(
            found.iter().any(|line| line.contains("never change")),
            "{found:?}"
        );
    }

    #[test]
    fn test_committed_migration_gate_blocks_uncommitted_content_edits_at_the_merge_base() {
        let repo = scratch_repo("iron-migrations-committed-mergebase");
        let file = repo.join("migrations/server/0001_init.sql");
        std::fs::write(&file, "CREATE TABLE a (id BIGINT);\n").unwrap();
        commit(&repo);
        git(&repo, &["branch", "dev"]);
        git(&repo, &["checkout", "-b", "ws/fixture"]);
        std::fs::write(&file, "CREATE TABLE a (id BIGINT PRIMARY KEY);\n").unwrap();
        let base = merge_base(&repo).trim().to_owned();
        let found = violations(&repo, &base);
        assert!(
            found.iter().any(|line| line.contains("never change")),
            "{found:?}"
        );
    }

    fn merge_base(repo: &Path) -> String {
        git_output(repo, &["merge-base", "HEAD", "dev"]).unwrap()
    }

    #[test]
    fn test_committed_migration_gate_allows_new_files_and_unchanged_content() {
        let repo = scratch_repo("iron-migrations-committed-add");
        std::fs::write(
            repo.join("migrations/server/0001_init.sql"),
            "CREATE TABLE a (id BIGINT);\n",
        )
        .unwrap();
        std::fs::write(
            repo.join("migrations/client/0001_init.sql"),
            "CREATE TABLE a (id TEXT);\n",
        )
        .unwrap();
        commit(&repo);
        std::fs::write(
            repo.join("migrations/server/0102_next.sql"),
            "CREATE TABLE b (id BIGINT);\n",
        )
        .unwrap();
        let found = violations(&repo, "HEAD");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn test_committed_migration_gate_blocks_deleting_a_committed_file() {
        let repo = scratch_repo("iron-migrations-committed-delete");
        std::fs::write(
            repo.join("migrations/server/0001_init.sql"),
            "CREATE TABLE a (id BIGINT);\n",
        )
        .unwrap();
        commit(&repo);
        std::fs::remove_file(repo.join("migrations/server/0001_init.sql")).unwrap();
        let found = violations(&repo, "HEAD");
        assert!(
            found.iter().any(|line| line.contains("missing locally")),
            "{found:?}"
        );
    }
}
