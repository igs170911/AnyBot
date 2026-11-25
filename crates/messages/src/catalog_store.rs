use std::cell::RefCell;
use std::collections::BTreeMap;

use fluent_bundle::{FluentBundle, FluentResource};

use crate::catalog::MessageCodeLike;
use crate::catalog::{EN_FTL, MessagesError, ZH_TW_FTL};
use crate::lang::{Lang, accepted_lang_tags};
use crate::params::Params;

use super::format::{check_param_set, message_pattern, to_fluent_args, wanted_names};

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
        let borrowed = bundle.borrow();
        let id = code.as_str();
        let pattern = message_pattern(&borrowed, id, lang)?;
        let wanted = wanted_names(pattern);
        check_param_set(id, &wanted, params)?;
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
        let pattern = message_pattern(&borrowed, code.as_str(), lang)?;
        Ok(wanted_names(pattern))
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
