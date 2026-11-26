use std::path::Path;

use crate::collect::read_file_lossy;
use crate::migrations_rules::statement_violations;
use crate::migrations_text::statements;
use crate::migrations_text::strip_sql_comments;

pub const MIGRATION_DIRS: [&str; 2] = ["migrations/server", "migrations/client"];

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for rel_dir in MIGRATION_DIRS {
        out.extend(dir_violations(repo, rel_dir));
    }
    out
}

fn dir_violations(repo: &Path, rel_dir: &str) -> Vec<String> {
    let dir = repo.join(rel_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![format!(
            "{rel_dir}: cannot be read; migrations must stay additive (spec/09 K118 6)"
        )];
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "sql") {
            files.push(path);
        }
    }
    files.sort();
    let mut out = Vec::new();
    for path in files {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let rel = format!("{rel_dir}/{name}");
        match read_file_lossy(&path) {
            Ok(content) => out.extend(file_violations(&rel, &content)),
            Err(e) => out.push(e),
        }
    }
    out
}

#[must_use]
pub fn file_violations(rel: &str, content: &str) -> Vec<String> {
    let stripped = strip_sql_comments(content);
    let mut out = Vec::new();
    for statement in statements(&stripped) {
        out.extend(statement_violations(rel, &statement));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn scratch_repo(name: &str, rel_dir: &str) -> PathBuf {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/scratch")
            .join(name);
        std::fs::create_dir_all(repo.join(rel_dir)).unwrap();
        repo
    }

    fn blocked(reason: &str, sql: &str) {
        let found = file_violations("migrations/server/fixture.sql", sql);
        assert!(
            found.iter().any(|line| line.contains(reason)),
            "expected {reason} in {found:?}"
        );
    }

    #[test]
    fn test_migration_gate_reads_sql_files_in_both_dirs() {
        let repo = scratch_repo("iron-migrations-dirs", "migrations/client");
        let sql = repo.join("migrations/client").join("0009_drops.sql");
        std::fs::write(&sql, "DROP TABLE notes;\n").unwrap();
        let found = violations(&repo);
        assert!(
            found
                .iter()
                .any(|line| line.contains("migrations/client/0009_drops.sql")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|line| line.contains("migrations/server")),
            "{found:?}"
        );
    }

    #[test]
    fn test_migration_gate_blocks_every_forbidden_shape() {
        blocked("drops an object", "ALTER TABLE t DROP COLUMN c;\n");
        blocked("drops an object", "DROP INDEX t_i;\n");
        blocked("drops an object", "ALTER TABLE t DROP CONSTRAINT t_c;\n");
        blocked(
            "drops an object",
            "ALTER TABLE t ALTER COLUMN c DROP DEFAULT;\n",
        );
        blocked(
            "drops an object",
            "ALTER TABLE t ALTER COLUMN c DROP NOT NULL;\n",
        );
        blocked("renames", "ALTER TABLE t RENAME TO u;\n");
        blocked("renames", "ALTER TABLE t RENAME COLUMN a TO b;\n");
        blocked(
            "changes a column type",
            "ALTER TABLE t ALTER COLUMN c TYPE BIGINT;\n",
        );
        blocked(
            "changes a column type",
            "ALTER TABLE t ALTER COLUMN c SET DATA TYPE TEXT;\n",
        );
        blocked(
            "required column with no default",
            "ALTER TABLE t ADD COLUMN c TEXT NOT NULL;\n",
        );
        blocked(
            "makes an existing column required",
            "ALTER TABLE t ALTER COLUMN c SET NOT NULL;\n",
        );
        blocked("constraint", "ALTER TABLE t ADD CONSTRAINT u UNIQUE (a);\n");
        blocked("constraint", "ALTER TABLE t ADD PRIMARY KEY (a);\n");
        blocked("constraint", "ALTER TABLE t ADD UNIQUE (a);\n");
        blocked(
            "constraint",
            "ALTER TABLE t ADD COLUMN b BIGINT REFERENCES u (id) NOT NULL DEFAULT 0;\n",
        );
        blocked("constraint", "ALTER TABLE t ADD CHECK (a > 0);\n");
        blocked(
            "constraint",
            "ALTER TABLE t ADD FOREIGN KEY (a) REFERENCES u (id);\n",
        );
        blocked(
            "constraint",
            "ALTER TABLE t ADD COLUMN b BIGINT REFERENCES u (id);\n",
        );
        blocked("unique index", "CREATE UNIQUE INDEX t_a_idx ON t (a);\n");
        blocked("removes data", "UPDATE t SET a = 1;\n");
        blocked("removes data", "DELETE FROM t WHERE a = 1;\n");
        blocked("removes data", "TRUNCATE TABLE t;\n");
        blocked("allow list", "CREATE SEQUENCE t_seq;\n");
        blocked("allow list", "GRANT SELECT ON t TO anyone;\n");
        blocked("non-adding clause", "ALTER TABLE t OWNER TO admin;\n");
    }

    #[test]
    fn test_migration_gate_allows_the_permitted_shapes() {
        let sql = "\
CREATE TABLE notes (
    id BIGINT PRIMARY KEY,
    kind TEXT NOT NULL UNIQUE,
    owner BIGINT REFERENCES accounts (id) CHECK (kind <> ''),
    type TEXT
);
CREATE INDEX notes_kind_idx ON notes (kind);
ALTER TABLE notes ADD COLUMN note TEXT;
ALTER TABLE notes ADD COLUMN note2 TEXT NOT NULL DEFAULT '';
INSERT INTO notes (id, kind) VALUES (1, 'seed');
INSERT INTO notes (id, kind) VALUES (2, 'drop table is just text');
";
        let found = file_violations("migrations/server/0001_init.sql", sql);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn test_migration_gate_ignores_words_in_comments_and_strings() {
        let sql = "\
-- DROP TABLE notes
/* UPDATE notes SET x = 1 */
INSERT INTO notes (id, kind) VALUES (1, 'DELETE FROM notes');
";
        let found = file_violations("migrations/server/0001_init.sql", sql);
        assert!(found.is_empty(), "{found:?}");
    }
}
