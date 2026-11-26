use sqlparser::ast::{Expr, Insert, SetExpr, TableObject, Value, ValueWithSpan, Values};

#[must_use = "each clause decides whether the seed insert stays fixed"]
pub fn insert_violations(rel: &str, line: usize, insert: &Insert) -> Vec<String> {
    let mut out = Vec::new();
    if matches!(insert.table, TableObject::TableFunction(_)) {
        out.push(violation(rel, line, "inserts into a table function"));
    }
    let fixed_rows = match insert.source.as_deref() {
        Some(query) if matches!(query.body.as_ref(), SetExpr::Values(_)) => {
            let SetExpr::Values(values) = query.body.as_ref() else {
                return not_fixed(rel, line);
            };
            !values.explicit_row && rows_are_constants(values)
        }
        _ => false,
    };
    if !fixed_rows {
        out.push(violation(
            rel,
            line,
            "inserts rows that are not fixed values",
        ));
    }
    let plain = insert.on.is_none()
        && insert.or.is_none()
        && !insert.ignore
        && insert.assignments.is_empty()
        && insert.returning.is_none()
        && !insert.overwrite
        && !insert.replace_into;
    let rows_are_plain = insert.source.as_ref().is_some_and(|query| {
        query.with.is_none() && query.order_by.is_none() && query.limit.is_none()
    });
    if !plain || !rows_are_plain {
        out.push(violation(rel, line, "inserts with extra clauses"));
    }
    out
}

fn rows_are_constants(values: &Values) -> bool {
    values
        .rows
        .iter()
        .all(|row| row.iter().all(expr_is_constant))
}

fn expr_is_constant(expr: &Expr) -> bool {
    matches!(expr, Expr::Value(value) if value_is_constant(value))
}

fn value_is_constant(value: &ValueWithSpan) -> bool {
    match &value.value {
        Value::Null
        | Value::Boolean(_)
        | Value::Number(_, _)
        | Value::SingleQuotedString(_)
        | Value::DoubleQuotedString(_)
        | Value::EscapedStringLiteral(_)
        | Value::UnicodeStringLiteral(_)
        | Value::DollarQuotedString(_) => true,
        Value::TripleSingleQuotedString(_)
        | Value::TripleDoubleQuotedString(_)
        | Value::SingleQuotedByteStringLiteral(_)
        | Value::DoubleQuotedByteStringLiteral(_)
        | Value::TripleSingleQuotedByteStringLiteral(_)
        | Value::TripleDoubleQuotedByteStringLiteral(_)
        | Value::SingleQuotedRawStringLiteral(_)
        | Value::DoubleQuotedRawStringLiteral(_)
        | Value::TripleSingleQuotedRawStringLiteral(_)
        | Value::TripleDoubleQuotedRawStringLiteral(_)
        | Value::NationalStringLiteral(_)
        | Value::HexStringLiteral(_)
        | Value::Placeholder(_) => false,
    }
}

fn not_fixed(rel: &str, line: usize) -> Vec<String> {
    vec![violation(
        rel,
        line,
        "inserts rows that are not fixed values",
    )]
}

fn violation(rel: &str, line: usize, reason: &str) -> String {
    format!(
        "{rel}:{line}: migration statement {reason}; migrations only add (spec/05 section 2, spec/09 K118 6)"
    )
}

#[cfg(test)]
mod tests {
    use crate::migrations_parser::parse_sql;

    fn violations_of(sql: &str) -> Vec<String> {
        let statements = parse_sql(sql).unwrap();
        let mut out = Vec::new();
        for statement in &statements {
            out.extend(crate::migrations_rules::statement_violations(
                "f.sql", 1, statement,
            ));
        }
        out
    }

    #[test]
    fn test_migration_gate_allows_fixed_seed_rows() {
        assert!(violations_of("INSERT INTO t (a) VALUES (1), (2);").is_empty());
        assert!(violations_of("INSERT INTO t (a, b) VALUES ('x', TRUE), (NULL, 'y');").is_empty());
    }

    #[test]
    fn test_migration_gate_blocks_insert_rows_that_are_not_constants() {
        assert!(
            violations_of("INSERT INTO t (a) VALUES ((SELECT b FROM u));")
                .iter()
                .any(|line| line.contains("not fixed values")),
            "subquery row must block"
        );
        assert!(
            violations_of("INSERT INTO t (a) VALUES (now());")
                .iter()
                .any(|line| line.contains("not fixed values")),
            "function row must block"
        );
        assert!(
            violations_of("INSERT INTO t (a) VALUES (1 + 2);")
                .iter()
                .any(|line| line.contains("not fixed values")),
            "expression row must block"
        );
    }

    #[test]
    fn test_migration_gate_blocks_insert_select_and_extra_clauses() {
        assert!(
            violations_of("INSERT INTO t (a) SELECT b FROM u;")
                .iter()
                .any(|line| line.contains("not fixed values")),
            "insert select must block"
        );
        assert!(
            violations_of("INSERT INTO t (a) VALUES (1) ON CONFLICT DO NOTHING;")
                .iter()
                .any(|line| line.contains("extra clauses")),
            "extra clauses must block"
        );
        assert!(
            violations_of("INSERT INTO t (a) VALUES (1) RETURNING a;")
                .iter()
                .any(|line| line.contains("extra clauses")),
            "returning must block"
        );
        assert!(
            violations_of("WITH cte AS (SELECT 1) INSERT INTO t (a) VALUES (1);")
                .iter()
                .any(|line| line.contains("allow list")),
            "cte must block"
        );
    }
}
