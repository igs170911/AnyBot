use std::fmt::{Debug, Display};

use unic_langid::LanguageIdentifier;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Lang {
    En,
    ZhTw,
}

pub const LANG_EN_TAG: &str = "en";
pub const LANG_ZH_TW_TAG: &str = "zh-TW";
pub const DEFAULT_LANG_TAG: &str = LANG_EN_TAG;

impl Lang {
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Lang::En => LANG_EN_TAG,
            Lang::ZhTw => LANG_ZH_TW_TAG,
        }
    }

    #[must_use]
    pub fn identifier(self) -> LanguageIdentifier {
        match self {
            Lang::En => unic_langid::langid!("en"),
            Lang::ZhTw => unic_langid::langid!("zh-TW"),
        }
    }

    pub fn parse(tag: &str) -> Result<Self, LangParseError> {
        match tag {
            LANG_EN_TAG => Ok(Self::En),
            LANG_ZH_TW_TAG => Ok(Self::ZhTw),
            _ => Err(LangParseError {
                given: tag.to_owned(),
            }),
        }
    }

    #[must_use]
    pub fn default_lang() -> Self {
        Self::En
    }
}

impl Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.tag())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct LangParseError {
    pub given: String,
}

impl Display for LangParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unsupported language tag {given} (supported: {en}, {zh})",
            given = self.given,
            en = LANG_EN_TAG,
            zh = LANG_ZH_TW_TAG,
        )
    }
}

impl std::error::Error for LangParseError {}

#[must_use]
pub fn accepted_lang_tags() -> Vec<&'static str> {
    vec![LANG_EN_TAG, LANG_ZH_TW_TAG]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supported_tags_cover_every_lang_variant() {
        let tags = accepted_lang_tags();
        assert_eq!(tags, vec![LANG_EN_TAG, LANG_ZH_TW_TAG]);
        for tag in tags {
            assert!(Lang::parse(tag).is_ok());
        }
    }

    #[test]
    fn test_lang_parse_accepts_supported_tags_and_default_is_en() {
        supported_tags_cover_every_lang_variant();
        assert_eq!(Lang::default_lang(), Lang::En);
        assert_eq!(Lang::parse("en"), Ok(Lang::En));
        assert_eq!(Lang::parse("zh-TW"), Ok(Lang::ZhTw));
    }

    #[test]
    fn test_lang_parse_rejects_unknown_tag_with_listing() {
        let err = Lang::parse("de").unwrap_err();
        assert_eq!(err.given, "de");
        let text = format!("{err}");
        assert!(text.contains("en"));
        assert!(text.contains("zh-TW"));
    }

    #[test]
    fn test_lang_identifier_matches_tag() {
        assert_eq!(Lang::En.identifier().to_string(), "en");
        assert_eq!(Lang::ZhTw.identifier().to_string(), "zh-TW");
    }

    #[test]
    fn lang_identifier_is_compiled_from_tag() {
        assert_eq!(Lang::En.identifier(), unic_langid::langid!("en"));
        assert_eq!(Lang::ZhTw.identifier(), unic_langid::langid!("zh-TW"));
        assert_eq!(Lang::En.identifier().language.as_str(), "en");
    }
}
