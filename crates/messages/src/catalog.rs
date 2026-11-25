use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Debug;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use thiserror::Error;

use crate::lang::{Lang, accepted_lang_tags};
use crate::params::{ParamValue, Params};

pub const EN_FTL: &str = include_str!("../../../locales/en.ftl");
pub const ZH_TW_FTL: &str = include_str!("../../../locales/zh-TW.ftl");

pub trait MessageCodeLike {
    fn as_str(&self) -> &str;
}

#[derive(Error, Debug, PartialEq)]
pub enum MessagesError {
    #[error("message code {code} not found in the {lang} bundle")]
    MissingMessage { code: String, lang: String },
    #[error("formatting message {code} failed: {reason}")]
    FormatFailure { code: String, reason: String },
    #[error("language {lang} has no embedded bundle")]
    MissingBundle { lang: String },
}

#[derive(Default)]
struct BundleStore(BTreeMap<Lang, RefCell<FluentBundle<FluentResource>>>);

impl BundleStore {
    fn build_bundle(lang: Lang) -> Result<FluentBundle<FluentResource>, MessagesError> {
        let source = match lang {
            Lang::En => EN_FTL,
            Lang::ZhTw => ZH_TW_FTL,
        };
        let resource = FluentResource::try_new(source.to_owned()).map_err(|e| {
            MessagesError::FormatFailure {
                code: lang.tag().to_owned(),
                reason: format!("ftl parse failed: {}", e.1.len()),
            }
        })?;
        let mut bundle = FluentBundle::new(vec![lang.identifier()]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .map_err(|errors| MessagesError::FormatFailure {
                code: lang.tag().to_owned(),
                reason: format!("ftl resource rejected: {} entries", errors.len()),
            })?;
        Ok(bundle)
    }

    fn bundle(
        &mut self,
        lang: Lang,
    ) -> Result<&RefCell<FluentBundle<FluentResource>>, MessagesError> {
        if !accepted_lang_tags().contains(&lang.tag()) {
            return Err(MessagesError::MissingBundle {
                lang: lang.tag().to_owned(),
            });
        }
        let built = Self::build_bundle(lang)?;
        let entry = self.0.entry(lang).or_insert_with(|| RefCell::new(built));
        Ok(entry)
    }
}

pub struct Catalog(BundleStore);

impl Catalog {
    #[must_use]
    pub fn new() -> Self {
        Self(BundleStore::default())
    }

    pub fn format<C: MessageCodeLike>(
        &mut self,
        lang: Lang,
        code: &C,
        params: &Params,
    ) -> Result<String, MessagesError> {
        let store = &mut self.0;
        let bundle = store.bundle(lang)?;
        let id = code.as_str();
        let borrowed = bundle.borrow();
        let message = borrowed
            .get_message(id)
            .ok_or_else(|| MessagesError::MissingMessage {
                code: id.to_owned(),
                lang: lang.tag().to_owned(),
            })?;
        let pattern = message
            .value()
            .ok_or_else(|| MessagesError::MissingMessage {
                code: id.to_owned(),
                lang: lang.tag().to_owned(),
            })?;
        let args = to_fluent_args(params);
        let mut errors = Vec::new();
        let text = borrowed.format_pattern(pattern, args.as_ref(), &mut errors);
        if !errors.is_empty() {
            return Err(MessagesError::FormatFailure {
                code: id.to_owned(),
                reason: format!("{} resolver errors", errors.len()),
            });
        }
        Ok(text.into_owned())
    }

    pub fn format_en<C: MessageCodeLike>(
        &mut self,
        code: &C,
        params: &Params,
    ) -> Result<String, MessagesError> {
        self.format(Lang::En, code, params)
    }

    pub fn format_str(
        &mut self,
        lang: Lang,
        code: &str,
        params: &Params,
    ) -> Result<String, MessagesError> {
        self.format(lang, &RawCode(code), params)
    }

    pub fn format_default<C: MessageCodeLike>(
        &mut self,
        code: &C,
        params: &Params,
    ) -> Result<String, MessagesError> {
        self.format(Lang::default_lang(), code, params)
    }

    pub fn param_names<C: MessageCodeLike>(
        &mut self,
        lang: Lang,
        code: &C,
    ) -> Result<Vec<String>, MessagesError> {
        let store = &mut self.0;
        let bundle = store.bundle(lang)?;
        let borrowed = bundle.borrow();
        let id = code.as_str();
        let message = borrowed
            .get_message(id)
            .ok_or_else(|| MessagesError::MissingMessage {
                code: id.to_owned(),
                lang: lang.tag().to_owned(),
            })?;
        let pattern = message
            .value()
            .ok_or_else(|| MessagesError::MissingMessage {
                code: id.to_owned(),
                lang: lang.tag().to_owned(),
            })?;
        let mut names = Vec::new();
        crate::names::collect_names(pattern, &mut names);
        names.sort();
        names.dedup();
        Ok(names)
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

struct RawCode<'a>(&'a str);

impl MessageCodeLike for RawCode<'_> {
    fn as_str(&self) -> &str {
        self.0
    }
}

fn to_fluent_args(params: &Params) -> Option<FluentArgs<'_>> {
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
    use crate::code::TestCode;

    fn code(text: &str) -> TestCode {
        TestCode::new(text)
    }

    #[test]
    fn test_catalog_formats_en_with_number_and_text_params() {
        let mut catalog = Catalog::new();
        let params = Params::new()
            .with_int("port", 8787)
            .with_text("db_host", "localhost")
            .with_text("db_name", "anybot")
            .with_text("version", "0.1.0");
        let got = catalog
            .format_en(&code("server-startup-listening"), &params)
            .unwrap();
        assert!(got.contains("127.0.0.1:8787"));
        assert!(got.contains("[::1]:8787"));
        assert!(got.contains("localhost/anybot"));
        assert!(got.contains("v0.1.0"));
    }

    #[test]
    fn test_catalog_formats_zh_tw_spec_wording_with_escapes() {
        let mut catalog = Catalog::new();
        let params = Params::new()
            .with_int("port", 8787)
            .with_text("db_host", "localhost")
            .with_text("db_name", "anybot")
            .with_text("version", "0.1.0");
        let got = catalog
            .format(Lang::ZhTw, &code("server-startup-listening"), &params)
            .unwrap();
        let expected = "AnyBot \u{4f3a}\u{670d}\u{52d9}\u{5668} v0.1.0\u{ff1a}\u{807d} 127.0.0.1:8787\u{3001}[::1]:8787\u{ff08}\u{8cc7}\u{6599}\u{5eab} localhost/anybot\u{ff09}";
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
    fn test_catalog_missing_param_reports_format_failure() {
        let mut catalog = Catalog::new();
        let err = catalog
            .format_en(&code("server-startup-listening"), &Params::new())
            .unwrap_err();
        assert!(matches!(err, MessagesError::FormatFailure { .. }));
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
