use sqlparser::ast::{AlterTableOperation, ColumnOption, Statement};

use crate::migrations_insert::insert_violations;

#[must_use = "each statement decides whether the migration file stays on the allow list"]
pub fn statement_violations(rel: &str, line: usize, statement: &Statement) -> Vec<String> {
    if !is_allow_listed(statement) {
        return vec![violation(
            rel,
            line,
            &format!("is not on the migration allow list: {statement}"),
        )];
    }
    let mut out = Vec::new();
    if let Statement::CreateTable(created) = statement {
        out.extend(create_table_violations(rel, line, created));
    }
    if let Statement::CreateIndex(index) = statement
        && index.unique
    {
        out.push(violation(rel, line, "creates a unique index"));
    }
    if let Statement::AlterTable { operations, .. } = statement {
        out.extend(alter_violations(rel, line, operations));
    }
    if let Statement::Insert(insert) = statement {
        out.extend(insert_violations(rel, line, insert));
    }
    out
}

#[must_use = "the allow list is the set of shapes a migration file may contain"]
fn is_allow_listed(statement: &Statement) -> bool {
    matches!(
        statement,
        Statement::CreateTable(_)
            | Statement::CreateIndex(_)
            | Statement::AlterTable { .. }
            | Statement::Insert(_)
    )
}

fn create_table_violations(
    rel: &str,
    line: usize,
    created: &sqlparser::ast::CreateTable,
) -> Vec<String> {
    let mut out = Vec::new();
    if created.query.is_some() {
        out.push(violation(rel, line, "creates a table from a query"));
    }
    if created.like.is_some() || created.clone.is_some() {
        out.push(violation(rel, line, "creates a table by copying another"));
    }
    if created.columns.is_empty() && created.constraints.is_empty() {
        out.push(violation(rel, line, "creates a table with no columns"));
    }
    out
}

fn alter_violations(rel: &str, line: usize, operations: &[AlterTableOperation]) -> Vec<String> {
    let mut out = Vec::new();
    for operation in operations {
        if let AlterTableOperation::AddColumn { column_def, .. } = operation {
            out.extend(add_column_violations(rel, line, column_def));
        } else {
            out.push(violation(
                rel,
                line,
                &format!("alters a table in a non-adding clause: {operation}"),
            ));
        }
    }
    out
}

fn add_column_violations(
    rel: &str,
    line: usize,
    column: &sqlparser::ast::ColumnDef,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut required = false;
    let mut has_default = false;
    for option in &column.options {
        if let Some(reason) = non_additive_reason(&option.option) {
            out.push(violation(rel, line, reason));
        }
        if matches!(option.option, ColumnOption::NotNull) {
            required = true;
        }
        if matches!(option.option, ColumnOption::Default(_)) {
            has_default = true;
        }
    }
    if required && !has_default {
        out.push(violation(
            rel,
            line,
            "adds a required column with no default",
        ));
    }
    out
}

#[must_use = "the reason names the column option that breaks the additive rule"]
fn non_additive_reason(option: &ColumnOption) -> Option<&'static str> {
    if matches!(
        option,
        ColumnOption::Null
            | ColumnOption::NotNull
            | ColumnOption::Default(_)
            | ColumnOption::Materialized(_)
            | ColumnOption::Ephemeral(_)
            | ColumnOption::Alias(_)
            | ColumnOption::DialectSpecific(_)
    ) {
        return None;
    }
    match option {
        ColumnOption::Unique { is_primary, .. } => {
            if *is_primary {
                Some("adds a primary key column")
            } else {
                Some("adds a unique column")
            }
        }
        ColumnOption::ForeignKey { .. } => Some("adds a foreign key column"),
        ColumnOption::Check(_) => Some("adds a check constraint"),
        ColumnOption::Generated { .. } | ColumnOption::Identity(_) => {
            Some("adds a generated column")
        }
        ColumnOption::CharacterSet(_) => Some("adds a character set"),
        ColumnOption::Collation(_) => Some("adds a collation"),
        ColumnOption::Comment(_) => Some("adds a comment clause"),
        ColumnOption::OnUpdate(_) => Some("adds an on-update clause"),
        ColumnOption::OnConflict(_) => Some("adds an on-conflict clause"),
        ColumnOption::Options(_) => Some("adds table options"),
        ColumnOption::Policy(_) => Some("adds a column policy"),
        ColumnOption::Tags(_) => Some("adds column tags"),
        other @ (ColumnOption::Null
        | ColumnOption::NotNull
        | ColumnOption::Default(_)
        | ColumnOption::Materialized(_)
        | ColumnOption::Ephemeral(_)
        | ColumnOption::Alias(_)
        | ColumnOption::DialectSpecific(_)) => {
            std::hint::black_box(other);
            None
        }
    }
}

fn violation(rel: &str, line: usize, reason: &str) -> String {
    format!(
        "{rel}:{line}: migration statement {reason}; migrations only add (spec/05 section 2, spec/09 K118 6)"
    )
}
