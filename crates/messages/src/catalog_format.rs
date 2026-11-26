use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use fluent_syntax::ast;

use crate::catalog::MessagesError;
use crate::lang::Lang;
use crate::params::{ParamValue, Params};

pub(super) fn message_pattern<'b>(
    bundle: &'b FluentBundle<FluentResource>,
    id: &str,
    lang: Lang,
) -> Result<&'b ast::Pattern<&'b str>, MessagesError> {
    let missing = || MessagesError::MissingMessage {
        code: id.to_owned(),
        lang: lang.tag().to_owned(),
    };
    let message = bundle.get_message(id).ok_or_else(missing)?;
    message.value().ok_or_else(missing)
}

pub(super) fn wanted_names(pattern: &ast::Pattern<&str>) -> Vec<String> {
    let mut names = Vec::new();
    crate::names::collect_names(pattern, &mut names);
    names.sort();
    names.dedup();
    names
}

pub(super) fn check_param_set(
    code: &str,
    wanted: &[String],
    params: &Params,
) -> Result<(), MessagesError> {
    let given: std::collections::BTreeSet<&str> = params.keys().into_iter().collect();
    let missing: Vec<String> = wanted
        .iter()
        .filter(|name| !given.contains(name.as_str()))
        .cloned()
        .collect();
    let extra: Vec<String> = given
        .iter()
        .filter(|key| !wanted.iter().any(|name| name == *key))
        .map(|key| (*key).to_owned())
        .collect();
    if missing.is_empty() && extra.is_empty() {
        return Ok(());
    }
    Err(MessagesError::ParamsMismatch {
        code: code.to_owned(),
        missing,
        extra,
    })
}
pub(super) fn to_fluent_args(params: &Params) -> Option<FluentArgs<'_>> {
    if params.is_empty() {
        return None;
    }
    let mut args = FluentArgs::new();
    for (key, value) in params.entries() {
        let fluent = match value {
            ParamValue::Int(number) => FluentValue::from(number),
            ParamValue::Text(text) => FluentValue::from(text),
        };
        args.set(key, fluent);
    }
    Some(args)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::code::TestCode;

    fn code(text: &str) -> TestCode {
        TestCode::new(text)
    }

    #[test]
    fn test_catalog_formats_en_with_number_and_text_params() {
        let mut catalog = Catalog::new();
        let params = Params::new()
            .with_text("addresses", "127.0.0.1:32411, [::1]:32411")
            .with_text("db_host", "localhost")
            .with_text("db_name", "anybot")
            .with_text("version", "0.1.0");
        let got = catalog
            .format_en(&code("server-startup-listening"), &params)
            .unwrap();
        assert!(got.contains("127.0.0.1:32411"));
        assert!(got.contains("[::1]:32411"));
        assert!(got.contains("localhost/anybot"));
        assert!(got.contains("v0.1.0"));
    }

    #[test]
    fn test_catalog_formats_zh_tw_spec_wording_with_escapes() {
        let mut catalog = Catalog::new();
        let params = Params::new()
            .with_text("addresses", "127.0.0.1:32411\u{3001}[::1]:32411")
            .with_text("db_host", "localhost")
            .with_text("db_name", "anybot")
            .with_text("version", "0.1.0");
        let got = catalog
            .format(Lang::ZhTw, &code("server-startup-listening"), &params)
            .unwrap();
        let expected = "AnyBot \u{4f3a}\u{670d}\u{52d9}\u{5668} v0.1.0\u{ff1a}\u{807d} 127.0.0.1:32411\u{3001}[::1]:32411\u{ff08}\u{8cc7}\u{6599}\u{5eab} localhost/anybot\u{ff09}";
        assert_eq!(got, expected);
    }

    #[test]
    fn test_catalog_missing_code_is_error_not_empty_string() {
        let mut catalog = Catalog::new();
        let err = catalog
            .format_en(&code("definitely-not-a-code"), &Params::new())
            .unwrap_err();
        assert_eq!(
            err,
            MessagesError::MissingMessage {
                code: "definitely-not-a-code".to_owned(),
                lang: "en".to_owned(),
            }
        );
    }

    #[test]
    fn test_catalog_message_without_params_formats_cleanly() {
        let mut catalog = Catalog::new();
        let got = catalog
            .format_en(&code("server-bootstrap-missing"), &Params::new())
            .unwrap();
        assert_eq!(
            got,
            "Bootstrap file not found (specify it with --bootstrap)"
        );
    }

    #[test]
    fn test_catalog_output_has_no_isolation_marks() {
        let mut catalog = Catalog::new();
        let params = Params::new()
            .with_int("port", 1)
            .with_text("reason", "address in use");
        let got = catalog
            .format(Lang::ZhTw, &code("server-port-unusable"), &params)
            .unwrap();
        assert!(!got.contains('\u{2066}'));
        assert!(!got.contains('\u{2069}'));
    }

    #[test]
    fn test_catalog_missing_param_reports_params_mismatch() {
        let mut catalog = Catalog::new();
        let err = catalog
            .format_en(&code("server-startup-listening"), &Params::new())
            .unwrap_err();
        assert_eq!(
            err,
            MessagesError::ParamsMismatch {
                code: "server-startup-listening".to_owned(),
                missing: vec![
                    "addresses".to_owned(),
                    "db_host".to_owned(),
                    "db_name".to_owned(),
                    "version".to_owned(),
                ],
                extra: vec![],
            }
        );
    }

    #[test]
    fn test_catalog_extra_param_reports_params_mismatch() {
        let mut catalog = Catalog::new();
        let params = Params::new()
            .with_int("version", 1)
            .with_text("addresses", "127.0.0.1:32411")
            .with_text("db_host", "localhost")
            .with_text("db_name", "anybot")
            .with_text("reason", "extra");
        let err = catalog
            .format_en(&code("server-startup-listening"), &params)
            .unwrap_err();
        assert_eq!(
            err,
            MessagesError::ParamsMismatch {
                code: "server-startup-listening".to_owned(),
                missing: vec![],
                extra: vec!["reason".to_owned()],
            }
        );
    }

    #[test]
    fn test_catalog_exact_param_set_formats() {
        let mut catalog = Catalog::new();
        let params = Params::new()
            .with_int("port", 31_313)
            .with_text("reason", "Address already in use");
        let got = catalog
            .format_en(&code("server-port-unusable"), &params)
            .unwrap();
        assert_eq!(got, "Cannot use port 31313: Address already in use");
    }

    #[test]
    fn test_catalog_default_lang_is_en() {
        let mut catalog = Catalog::new();
        let got = catalog
            .format_default(&code("server-bootstrap-missing"), &Params::new())
            .unwrap();
        assert_eq!(
            got,
            "Bootstrap file not found (specify it with --bootstrap)"
        );
    }
}
