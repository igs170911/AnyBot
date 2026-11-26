use std::path::Path;

use sqlparser::ast::Statement;

use crate::collect::read_file_lossy;
use crate::migrations_parser::parse_sql;
use crate::migrations_rules::statement_violations;

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

#[must_use = "the parsed syntax tree decides what the migration gate allows"]
pub fn file_violations(rel: &str, content: &str) -> Vec<String> {
    let statements = match parse_sql(content) {
        Ok(statements) => statements,
        Err(e) => {
            return vec![format!(
                "{rel}: {e}; migrations are checked by parsing (spec/09 K118 6)"
            )];
        }
    };
    let mut out = Vec::new();
    for statement in &statements {
        out.extend(statement_violations_of(rel, statement));
    }
    out
}

fn statement_violations_of(rel: &str, statement: &Statement) -> Vec<String> {
    statement_violations(rel, 1, statement)
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

    fn allowed(sql: &str) {
        let found = file_violations("migrations/server/fixture.sql", sql);
        assert!(found.is_empty(), "{found:?}");
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
    fn test_migration_gate_blocks_destructive_shapes() {
        blocked("DROP COLUMN c", "ALTER TABLE t DROP COLUMN c;\n");
        blocked("allow list", "DROP INDEX t_i;\n");
        blocked("DROP CONSTRAINT", "ALTER TABLE t DROP CONSTRAINT t_c;\n");
        blocked(
            "DROP DEFAULT",
            "ALTER TABLE t ALTER COLUMN c DROP DEFAULT;\n",
        );
        blocked(
            "DROP NOT NULL",
            "ALTER TABLE t ALTER COLUMN c DROP NOT NULL;\n",
        );
        blocked("RENAME", "ALTER TABLE t RENAME TO u;\n");
        blocked("RENAME", "ALTER TABLE t RENAME COLUMN a TO b;\n");
        blocked(
            "SET DATA TYPE",
            "ALTER TABLE t ALTER COLUMN c TYPE BIGINT;\n",
        );
        blocked(
            "SET DATA TYPE",
            "ALTER TABLE t ALTER COLUMN c SET DATA TYPE TEXT;\n",
        );
    }

    #[test]
    fn test_migration_gate_blocks_shapes_outside_the_allow_list() {
        blocked(
            "required column with no default",
            "ALTER TABLE t ADD COLUMN c TEXT NOT NULL;\n",
        );
        blocked(
            "SET NOT NULL",
            "ALTER TABLE t ALTER COLUMN c SET NOT NULL;\n",
        );
        blocked(
            "ADD CONSTRAINT",
            "ALTER TABLE t ADD CONSTRAINT u UNIQUE (a);\n",
        );
        blocked("ADD PRIMARY KEY", "ALTER TABLE t ADD PRIMARY KEY (a);\n");
        blocked("ADD UNIQUE", "ALTER TABLE t ADD UNIQUE (a);\n");
        blocked("ADD CHECK", "ALTER TABLE t ADD CHECK (a > 0);\n");
        blocked(
            "ADD FOREIGN KEY",
            "ALTER TABLE t ADD FOREIGN KEY (a) REFERENCES u (id);\n",
        );
        blocked(
            "creates a unique index",
            "CREATE UNIQUE INDEX t_a_idx ON t (a);\n",
        );
        blocked("allow list", "UPDATE t SET a = 1;\n");
        blocked("allow list", "DELETE FROM t WHERE a = 1;\n");
        blocked("allow list", "TRUNCATE TABLE t;\n");
        blocked("CREATE SEQUENCE", "CREATE SEQUENCE t_seq;\n");
        blocked("GRANT", "GRANT SELECT ON t TO anyone;\n");
        blocked("OWNER TO", "ALTER TABLE t OWNER TO admin;\n");
        blocked(
            "the sql parser rejects",
            "ALTER TABLE t SET WITHOUT OIDS;\n",
        );
        blocked("from a query", "CREATE TABLE b AS SELECT id FROM a;\n");
        blocked("by copying another", "CREATE TABLE b LIKE a;\n");
        blocked("not fixed values", "INSERT INTO t (a) SELECT b FROM u;\n");
        blocked("allow list", "CREATE VIEW v AS SELECT 1;\n");
        blocked(
            "non-adding clause",
            "CREATE TABLE a (id INT);\nALTER TABLE old ADD CONSTRAINT old_c CHECK (id > 0);\n",
        );
    }

    #[test]
    fn test_migration_gate_blocks_the_reviewer_bypass_attempts() {
        blocked(
            "required column with no default",
            "ALTER TABLE t ADD COLUMN default_model TEXT NOT NULL;\n",
        );
        blocked(
            "required column with no default",
            "ALTER TABLE t ADD COLUMN a TEXT NOT NULL DEFAULT '', ADD COLUMN b TEXT NOT NULL;\n",
        );
        blocked(
            "required column with no default",
            "ALTER TABLE t ADD c INT NOT NULL, ADD d INT;\n",
        );
        blocked(
            "ALTER COLUMN d SET DATA TYPE",
            "ALTER TABLE t ADD COLUMN c INT, ALTER d TYPE BIGINT;\n",
        );
        blocked(
            "SET NOT NULL",
            "ALTER TABLE t\nADD COLUMN\nn TEXT;\nALTER TABLE t\nALTER COLUMN n SET  NOT NULL;\n",
        );
        blocked(
            "unique index",
            "CREATE TABLE a (id INT); CREATE UNIQUE INDEX i ON old (c);\n",
        );
        blocked(
            "adds a required column with no default",
            "ALTER TABLE t ADD COLUMN IF NOT EXISTS n TEXT NOT NULL;\n",
        );
    }

    #[test]
    fn test_migration_gate_blocks_sql_it_cannot_parse() {
        blocked("the sql parser rejects", "ALTER TABLE t ADD COLUMN;\n");
        blocked("the sql parser rejects", "CREATE TABLE a (id INT\n");
        blocked("the sql parser rejects", "SEED DATA HERE;\n");
    }

    #[test]
    fn test_migration_gate_allows_the_permitted_shapes() {
        allowed(
            "CREATE TABLE notes (id BIGINT PRIMARY KEY, kind TEXT NOT NULL UNIQUE, owner BIGINT REFERENCES accounts (id) ON DELETE CASCADE CHECK (kind <> ''), type TEXT);\n",
        );
        allowed("CREATE INDEX notes_kind_idx ON notes (kind);\n");
        allowed("ALTER TABLE notes ADD COLUMN note TEXT;\n");
        allowed("ALTER TABLE notes ADD COLUMN note2 TEXT NOT NULL DEFAULT '';\n");
        allowed("ALTER TABLE notes ADD COLUMN note3 TEXT DEFAULT 'drop table is just text';\n");
        allowed("INSERT INTO notes (id, kind) VALUES (1, 'seed');\n");
        allowed("INSERT INTO notes (id, kind) VALUES (2, 'DELETE FROM notes');\n");
        allowed("INSERT INTO notes (id, kind) VALUES (3, $$ drop table $$);\n");
    }

    #[test]
    fn test_migration_gate_ignores_words_inside_strings() {
        allowed("INSERT INTO notes (id, kind) VALUES (1, 'DROP TABLE notes');\n");
        allowed("ALTER TABLE notes ADD COLUMN note TEXT DEFAULT 'update delete truncate';\n");
    }
}
