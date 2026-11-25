use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MessagesError {
    #[error("message code {code} not found in the {lang} bundle")]
    MissingMessage { code: String, lang: String },
    #[error("formatting message {code} failed: {reason}")]
    FormatFailure { code: String, reason: String },
    #[error("language {lang} has no embedded bundle")]
    MissingBundle { lang: String },
    #[error("params for message {code} do not match: missing {missing:?}, extra {extra:?}")]
    ParamsMismatch {
        code: String,
        missing: Vec<String>,
        extra: Vec<String>,
    },
}
