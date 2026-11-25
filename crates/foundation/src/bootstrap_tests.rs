use super::{DEFAULT_POSTGRES_PORT, parse_connection};

#[test]
fn test_bootstrap_missing_port_uses_postgres_default() {
    let parsed =
        parse_connection("postgresql://u:\u{5bc6}\u{7801}@localhost/\u{8cc7}\u{6599}\u{5eab}")
            .unwrap();
    assert_eq!(parsed.port, DEFAULT_POSTGRES_PORT);
    assert_eq!(parsed.host, "localhost");
    assert_eq!(parsed.database, "\u{8cc7}\u{6599}\u{5eab}");
}

#[test]
fn test_bootstrap_short_scheme_missing_port_uses_postgres_default() {
    let parsed = parse_connection("postgres://u:\u{5bc6}\u{7801}@db.example/anybot").unwrap();
    assert_eq!(parsed.port, DEFAULT_POSTGRES_PORT);
    assert_eq!(parsed.host, "db.example");
}

#[test]
fn test_bootstrap_explicit_port_is_kept() {
    let parsed = parse_connection("postgresql://u:\u{5bc6}\u{7801}@localhost:5433/anybot").unwrap();
    assert_eq!(parsed.port, 5433);
}

#[test]
fn test_bootstrap_rejects_missing_database_and_bad_scheme() {
    assert!(parse_connection("postgresql://u:\u{5bc6}\u{7801}@localhost").is_err());
    assert!(parse_connection("mysql://u:\u{5bc6}\u{7801}@localhost:3306/anybot").is_err());
    assert!(parse_connection("postgresql://u@localhost:5432/anybot").is_err());
    assert!(parse_connection("not a url").is_err());
}
