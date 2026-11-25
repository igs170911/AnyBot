pub(crate) fn permit_for(
    endpoint: &anybot_models::ProviderEndpoint,
) -> Result<anybot_models::SendPermit, String> {
    let local = anybot_models::DataPolicy::LocalOnly;
    let cloud = anybot_models::DataPolicy::Cloud {
        allowed: vec![endpoint.kind],
    };
    let use_kind = anybot_models::ModelUse::Bot;
    anybot_models::SendPermit::for_endpoint(&local, endpoint, use_kind)
        .or_else(|| anybot_models::SendPermit::for_endpoint(&cloud, endpoint, use_kind))
        .ok_or_else(|| "permit".to_owned())
}
