use crate::session::ModelRef;

impl ModelRef {
    #[must_use]
    pub fn copy_ref(&self) -> Self {
        copy_model(self)
    }
}

pub(crate) fn copy_model(model: &ModelRef) -> ModelRef {
    let key = match model.endpoint.api_key.as_ref() {
        Some(secret) => secret.expose(),
        None => "",
    };
    ModelRef {
        endpoint: crate::providers::resolve_endpoint(
            model.endpoint.kind,
            &model.endpoint.base_url,
            key,
            &model.endpoint.model,
            Some(model.endpoint.format),
        ),
        bot_context: model.bot_context,
    }
}
