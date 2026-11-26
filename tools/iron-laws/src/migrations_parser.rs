use sqlparser::ast::Statement;
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::{Parser, ParserError};

#[must_use = "the parsed statements decide what the migration gate allows"]
pub fn parse_sql(text: &str) -> Result<Vec<Statement>, String> {
    let dialect = PostgreSqlDialect {};
    match Parser::parse_sql(&dialect, text) {
        Ok(statements) => Ok(statements),
        Err(e) => Err(parse_error(&e)),
    }
}

fn parse_error(e: &ParserError) -> String {
    format!("the sql parser rejects the file: {e}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_gate_parses_two_statements_on_one_line() {
        let found = parse_sql("CREATE TABLE a (id INT); CREATE UNIQUE INDEX i ON old (c);");
        let statements = found.unwrap();
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_migration_gate_blocks_sql_it_cannot_parse() {
        assert!(parse_sql("ALTER TABLE t ADD COLUMN;").is_err());
        assert!(parse_sql("SELECT a WHERE b FROM c;").is_err());
        assert!(parse_sql("CREATE TABLE a (id INT").is_err());
    }

    #[test]
    fn test_migration_gate_parses_dollar_quotes_and_open_quotes_safely() {
        let found = parse_sql(
            "CREATE FUNCTION f() RETURNS text AS $$ DROP TABLE notes; $$ LANGUAGE sql;\nCREATE TABLE a (id INT);\n",
        );
        assert!(found.is_ok());
    }
}
