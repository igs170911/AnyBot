use anybot_foundation::{FoundationError, MessageRef};
use anybot_messages::{Lang, MessageCode, MessagesError, Params};

pub fn params_from_ref(reference: &MessageRef) -> Result<Params, anybot_messages::ParamsError> {
    let value =
        serde_json::to_value(&reference.params).map_err(|err| anybot_messages::ParamsError {
            reason: format!("message params must serialize: {err}"),
        })?;
    Params::from_json_value(&value)
}

pub fn code_from_str(text: &str) -> Result<MessageCode, MessagesError> {
    MessageCode::from_code(text).ok_or(MessagesError::MissingMessage {
        code: text.to_owned(),
        lang: Lang::En.tag().to_owned(),
    })
}

pub fn foundation_code(error: &FoundationError) -> Result<MessageCode, MessagesError> {
    let reference = error.message_ref().ok_or(MessagesError::MissingMessage {
        code: "foundation error without a message ref".to_owned(),
        lang: Lang::En.tag().to_owned(),
    })?;
    code_from_str(&reference.code)
}

pub fn foundation_message(error: &FoundationError) -> Result<(MessageCode, Params), MessagesError> {
    let code = foundation_code(error)?;
    let reference = error.message_ref().ok_or(MessagesError::MissingMessage {
        code: "foundation error without a message ref".to_owned(),
        lang: Lang::En.tag().to_owned(),
    })?;
    let params = params_from_ref(&reference).map_err(|err| MessagesError::MissingMessage {
        code: format!("{code:?}: {err}"),
        lang: Lang::En.tag().to_owned(),
    })?;
    Ok((code, params))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anybot_foundation::{
        CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN, CODE_SERVER_STARTUP_LISTENING, MessageRef,
    };
    use anybot_messages::Catalog;

    fn text_value(text: &str) -> anybot_messages::ParamValue {
        anybot_messages::ParamValue::text(text)
    }

    #[test]
    fn bridge_startup_code_converts_from_foundation_constant() {
        assert_eq!(
            code_from_str(CODE_SERVER_STARTUP_LISTENING).unwrap(),
            MessageCode::ServerStartupListening
        );
    }

    #[test]
    fn every_foundation_code_constant_converts_to_a_message_code() {
        for constant in anybot_foundation::FOUNDATION_MESSAGE_CODES {
            let code = code_from_str(constant).unwrap_or_else(|err| panic!("{constant}: {err}"));
            assert_eq!(code.as_str(), constant);
        }
    }

    #[test]
    fn bridge_rejects_unknown_code() {
        assert!(code_from_str("not-a-code").is_err());
    }

    #[test]
    fn bridge_params_roundtrip_from_reference() {
        let reference = MessageRef::new(CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN)
            .with_text("mode", "644")
            .with_text("path", "boot.conf");
        let params = params_from_ref(&reference).unwrap();
        assert_eq!(params.get("mode"), Some(&text_value("644")));
    }

    #[test]
    fn bridge_maps_bootstrap_permissions_error() {
        let error = FoundationError::BootstrapPermissions(
            MessageRef::new(CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN)
                .with_text("mode", "644")
                .with_text("path", "boot.conf"),
        );
        let (code, params) = foundation_message(&error).unwrap();
        assert_eq!(code, MessageCode::ServerBootstrapPermissionsTooOpen);
        assert_eq!(params.get("mode"), Some(&text_value("644")));
    }

    #[test]
    fn every_generated_code_formats_in_english() {
        let mut catalog = Catalog::new();
        for code in MessageCode::all() {
            let params = sample_params(&mut catalog, code);
            let text = catalog.format(Lang::En, &code, &params).unwrap();
            assert!(!text.is_empty(), "{code:?}");
        }
    }

    fn sample_params(catalog: &mut Catalog, wanted: MessageCode) -> Params {
        let mut params = Params::new();
        for name in catalog.param_names(Lang::En, &wanted).unwrap() {
            params = params.with_text(&name, "sample");
        }
        params
    }
}
