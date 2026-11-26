use std::path::Path;

use crate::collect::read_file_lossy;

pub const SERVER_MIGRATIONS_REL: &str = "migrations/server";

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let dir = repo.join(SERVER_MIGRATIONS_REL);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![format!(
            "{SERVER_MIGRATIONS_REL}: cannot be read; server migrations must stay additive"
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
        let rel = format!("{SERVER_MIGRATIONS_REL}/{name}");
        let Ok(content) = read_file_lossy(&path) else {
            out.push(format!("{rel}: cannot be read"));
            continue;
        };
        out.extend(file_violations(&rel, &content));
    }
    out
}

#[must_use]
pub fn file_violations(rel: &str, content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let stripped = strip_sql_comments(content);
    for (index, line) in stripped.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let number = index.saturating_add(1);
        if is_drop(trimmed) {
            out.push(format!(
                "{rel}:{number}: migration drops an object; migrations only add (server-boot S3)"
            ));
        }
        if alters_column_type(trimmed) {
            out.push(format!(
                "{rel}:{number}: migration changes a column type; migrations only add (server-boot S3)"
            ));
        }
        if required_column_without_default(trimmed) {
            out.push(format!(
                "{rel}:{number}: new required column has no default; old servers could not write it (server-boot S3)"
            ));
        }
    }
    out
}

fn is_drop(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    upper
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| word == "DROP")
}

fn alters_column_type(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    upper.contains("ALTER") && (upper.contains(" TYPE ") || upper.contains(" SET DATA TYPE"))
}

fn required_column_without_default(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    if !upper.contains("ADD") || !upper.contains("COLUMN") || !upper.contains("NOT NULL") {
        return false;
    }
    if upper.contains("PRIMARY KEY") {
        return false;
    }
    !upper.contains("DEFAULT")
}

fn strip_sql_comments(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let bytes = content.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes.get(i) == Some(&b'-') && bytes.get(i.saturating_add(1)) == Some(&b'-') {
            while bytes.get(i).is_some_and(|b| *b != b'\n') {
                i = i.saturating_add(1);
            }
            continue;
        }
        if bytes.get(i) == Some(&b'\'') {
            out.push('\'');
            i = i.saturating_add(1);
            while let Some(b) = bytes.get(i) {
                out.push(char::from(*b));
                i = i.saturating_add(1);
                if *b == b'\'' {
                    if bytes.get(i) == Some(&b'\'') {
                        out.push('\'');
                        i = i.saturating_add(1);
                        continue;
                    }
                    break;
                }
            }
            continue;
        }
        let Some(byte) = bytes.get(i) else {
            break;
        };
        out.push(char::from(*byte));
        i = i.saturating_add(1);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn scratch_repo(name: &str) -> PathBuf {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/scratch")
            .join(name);
        std::fs::create_dir_all(repo.join(SERVER_MIGRATIONS_REL)).unwrap();
        repo
    }

    #[test]
    fn test_migration_gate_reads_sql_files_in_the_server_dir() {
        let repo = scratch_repo("iron-migrations-drop");
        let sql = repo.join(SERVER_MIGRATIONS_REL).join("0009_drops.sql");
        std::fs::write(&sql, "ALTER TABLE notes DROP COLUMN kept;\n").unwrap();
        let found = violations(&repo);
        let _removed = std::fs::remove_dir_all(&repo);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found.iter().any(|line| line.contains("drops")), "{found:?}");
    }

    #[test]
    fn test_migration_gate_blocks_drop_type_change_and_required_column() {
        let sql = "\
ALTER TABLE idempotency DROP CONSTRAINT idempotency_pkey;
ALTER TABLE auth_events ALTER COLUMN at_ms TYPE BIGINT;
ALTER TABLE idempotency ADD COLUMN scope TEXT NOT NULL;
-- DROP TABLE kept_in_comment
CREATE TABLE notes (id BIGINT PRIMARY KEY);
";
        let found = file_violations("migrations/server/fixture.sql", sql);
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found.iter().any(|line| line.contains("drops")), "{found:?}");
        assert!(
            found.iter().any(|line| line.contains("column type")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|line| line.contains("no default")),
            "{found:?}"
        );
    }

    #[test]
    fn test_migration_gate_allows_additive_statements() {
        let sql = "\
CREATE TABLE auth_events (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL,
    address TEXT NOT NULL
);
ALTER TABLE idempotency ADD COLUMN scope TEXT NOT NULL DEFAULT '';
CREATE INDEX auth_events_account_order_idx ON auth_events (kind, account, id);
";
        let found = file_violations("migrations/server/0001_init.sql", sql);
        assert!(found.is_empty(), "{found:?}");
    }
}
