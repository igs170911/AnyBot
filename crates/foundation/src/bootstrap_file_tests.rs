use super::parse_bootstrap;
use crate::error::{
    CODE_BOOTSTRAP_LINE_MALFORMED, CODE_BOOTSTRAP_MASTER_KEY_MALFORMED,
    CODE_BOOTSTRAP_NAME_DUPLICATE, CODE_BOOTSTRAP_NAME_MISSING, CODE_BOOTSTRAP_NAME_UNKNOWN,
};

const DATABASE: &str = "database=postgres://anybot:pw-not-secret@db.example:5432/anybot";

fn key_hex() -> String {
    "0123456789abcdef".repeat(4)
}

fn rejected(text: &str) -> (String, Option<serde_json::Value>, Option<serde_json::Value>) {
    let err = parse_bootstrap(text).unwrap_err();
    let reference = err.message_ref().unwrap();
    (
        reference.code.clone(),
        reference.params.get("name").cloned(),
        reference.params.get("line").cloned(),
    )
}

#[test]
fn two_lines_with_blank_lines_parse_into_the_bootstrap() {
    let text = format!("\n{DATABASE}\n\n  master_key={}  \n\n", key_hex());
    let parsed = parse_bootstrap(&text).unwrap();
    assert_eq!(parsed.host, "db.example");
    assert_eq!(parsed.database, "anybot");
    assert_eq!(parsed.password.expose(), "pw-not-secret");
    assert_eq!(
        parsed.master_key.expose_bytes().get(..2),
        Some([0x01, 0x23].as_slice())
    );
}

#[test]
fn the_old_single_url_line_is_refused() {
    let (code, _name, line) = rejected("postgres://anybot:pw@db.example:5432/anybot\n");
    assert_eq!(code, CODE_BOOTSTRAP_LINE_MALFORMED);
    assert_eq!(line, Some(serde_json::json!("1")));
    let (code, _name, _line) = rejected("postgres://anybot:pw@db.example:5432/anybot?ssl=on\n");
    assert_eq!(code, CODE_BOOTSTRAP_LINE_MALFORMED);
}

#[test]
fn each_broken_shape_names_the_entry_without_its_value() {
    let missing = rejected(&format!("{DATABASE}\n"));
    assert_eq!(missing.0, CODE_BOOTSTRAP_NAME_MISSING);
    assert_eq!(missing.1, Some(serde_json::json!("master_key")));
    let unknown = rejected(&format!("{DATABASE}\nmaster_key={}\nextra=1\n", key_hex()));
    assert_eq!(unknown.0, CODE_BOOTSTRAP_NAME_UNKNOWN);
    assert_eq!(unknown.1, Some(serde_json::json!("extra")));
    let twice = rejected(&format!(
        "{DATABASE}\n{DATABASE}\nmaster_key={}\n",
        key_hex()
    ));
    assert_eq!(twice.0, CODE_BOOTSTRAP_NAME_DUPLICATE);
    assert_eq!(twice.1, Some(serde_json::json!("database")));
    let short_hex: String = key_hex().chars().skip(1).collect();
    let short = rejected(&format!("{DATABASE}\nmaster_key={short_hex}\n"));
    assert_eq!(short.0, CODE_BOOTSTRAP_MASTER_KEY_MALFORMED);
    assert_eq!(short.1, Some(serde_json::json!("master_key")));
}

#[test]
fn errors_never_carry_the_password_or_the_key() {
    for text in [
        format!("{DATABASE}\n"),
        format!("{DATABASE}\nmaster_key={}\nextra=1\n", key_hex()),
        format!("{DATABASE}\nmaster_key={}x\n", key_hex()),
    ] {
        let err = parse_bootstrap(&text).unwrap_err();
        let shown = format!("{err} {:?}", err.message_ref());
        assert!(!shown.contains("pw-not-secret"), "{shown}");
        assert!(!shown.contains(&key_hex()), "{shown}");
    }
}
