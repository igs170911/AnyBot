use crate::migrations_text::Statement;

const DROP_WORDS: [&str; 1] = ["DROP"];
const RENAME_WORDS: [&str; 1] = ["RENAME"];
const REMOVE_DATA_WORDS: [&str; 3] = ["UPDATE", "DELETE", "TRUNCATE"];
const EXISTING_TABLE_CONSTRAINT_WORDS: [&str; 6] = [
    "CONSTRAINT",
    "PRIMARY",
    "UNIQUE",
    "CHECK",
    "FOREIGN",
    "REFERENCES",
];

#[must_use]
pub fn statement_violations(rel: &str, statement: &Statement) -> Vec<String> {
    let upper = statement.text.to_ascii_uppercase();
    let words: Vec<&str> = upper
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return vec![];
    }
    let create_table = starts_with(&words, &["CREATE", "TABLE"]);
    let mut out = Vec::new();
    push_word_violations(rel, statement, create_table, &mut out);
    push_clause_violations(rel, statement, &upper, &words, &mut out);
    push_head_violations(rel, statement, &words, &mut out);
    out
}

fn push_word_violations(
    rel: &str,
    statement: &Statement,
    create_table: bool,
    out: &mut Vec<String>,
) {
    if let Some(at) = find_word_line(statement, DROP_WORDS) {
        out.push(violation_at(rel, at, "drops an object"));
    }
    if let Some(at) = find_word_line(statement, RENAME_WORDS) {
        out.push(violation_at(rel, at, "renames an object"));
    }
    if let Some(at) = find_word_line(statement, REMOVE_DATA_WORDS) {
        out.push(violation_at(rel, at, "removes data"));
    }
    let add_constraint = "adds a constraint to an existing table";
    if !create_table && let Some(at) = find_word_line(statement, EXISTING_TABLE_CONSTRAINT_WORDS) {
        out.push(violation_at(rel, at, add_constraint));
    }
}

fn find_word_line<const N: usize>(statement: &Statement, banned: [&str; N]) -> Option<usize> {
    find_word_line_long(statement, &banned)
}

fn find_word_line_long(statement: &Statement, banned: &[&str]) -> Option<usize> {
    let upper = statement.text.to_ascii_uppercase();
    let mut line = statement.start_line;
    let mut token = String::new();
    for ch in upper.chars() {
        if ch == '\n' {
            if is_banned(&token, banned) {
                return Some(line);
            }
            token.clear();
            line = line.saturating_add(1);
            continue;
        }
        if ch.is_ascii_alphanumeric() {
            token.push(ch);
        } else if is_banned(&token, banned) {
            return Some(line);
        } else {
            token.clear();
        }
    }
    if is_banned(&token, banned) {
        return Some(line);
    }
    None
}

fn is_banned(token: &str, banned: &[&str]) -> bool {
    !token.is_empty() && banned.contains(&token)
}

fn push_clause_violations(
    rel: &str,
    statement: &Statement,
    upper: &str,
    words: &[&str],
    out: &mut Vec<String>,
) {
    if !words.contains(&"ALTER") {
        return;
    }
    if upper.contains("ALTER COLUMN") && (upper.contains("TYPE") || upper.contains("SET DATA")) {
        out.push(violation(rel, statement, "changes a column type"));
    }
    if upper.contains("SET NOT NULL") {
        out.push(violation(
            rel,
            statement,
            "makes an existing column required",
        ));
    }
    if upper.contains("ADD COLUMN")
        && upper.contains("NOT NULL")
        && !upper.contains("PRIMARY KEY")
        && !upper.contains("DEFAULT")
    {
        out.push(violation(
            rel,
            statement,
            "adds a required column with no default",
        ));
    }
}

fn push_head_violations(rel: &str, statement: &Statement, words: &[&str], out: &mut Vec<String>) {
    if starts_with(words, &["CREATE", "TABLE"]) || starts_with(words, &["INSERT"]) {
        return;
    }
    if starts_with(words, &["CREATE", "INDEX"]) {
        return;
    }
    if starts_with(words, &["CREATE", "UNIQUE", "INDEX"]) {
        out.push(violation(rel, statement, "creates a unique index"));
        return;
    }
    if starts_with(words, &["ALTER", "TABLE"]) {
        if !words.contains(&"ADD") || !words.contains(&"COLUMN") {
            out.push(violation(
                rel,
                statement,
                "alters a table in a non-adding clause",
            ));
        }
        return;
    }
    out.push(violation(
        rel,
        statement,
        "is not on the additive allow list",
    ));
}

fn starts_with(words: &[&str], prefix: &[&str]) -> bool {
    words.len() >= prefix.len() && words.iter().zip(prefix.iter()).all(|(w, p)| w == p)
}

fn violation(rel: &str, statement: &Statement, reason: &str) -> String {
    violation_at(rel, statement.start_line, reason)
}

fn violation_at(rel: &str, line: usize, reason: &str) -> String {
    format!(
        "{rel}:{line}: migration statement {reason}; migrations only add (spec/05 section 2, spec/09 K118 6)"
    )
}
