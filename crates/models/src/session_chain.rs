use crate::session::{MAX_FALLBACKS, ModelRef};
use crate::types::ProviderKind;

pub(crate) fn chain_of(
    primary: &ModelRef,
    fallbacks: &[ModelRef],
    pending: Option<&ModelRef>,
    adopted: Option<&ModelRef>,
) -> Vec<ModelRef> {
    let mut chain = vec![crate::model_copy::copy_model(primary)];
    for item in fallbacks.iter().take(MAX_FALLBACKS) {
        chain.push(crate::model_copy::copy_model(item));
    }
    if let Some(extra) = adopted
        && !in_chain(&chain, extra)
    {
        chain.insert(0, crate::model_copy::copy_model(extra));
    }
    if let Some(next) = pending
        && !in_chain(&chain, next)
    {
        chain.insert(0, crate::model_copy::copy_model(next));
    }
    prefer_pending(chain, pending.map(kind_model))
}

fn kind_model(model: &ModelRef) -> (ProviderKind, &str) {
    (model.endpoint.kind, model.endpoint.model.as_str())
}

fn in_chain(chain: &[ModelRef], model: &ModelRef) -> bool {
    chain.iter().any(|item| same_model(item, model))
}

fn same_model(left: &ModelRef, right: &ModelRef) -> bool {
    left.endpoint.kind == right.endpoint.kind && left.endpoint.model == right.endpoint.model
}

fn prefer_pending(chain: Vec<ModelRef>, pending: Option<(ProviderKind, &str)>) -> Vec<ModelRef> {
    let Some((kind, model)) = pending else {
        return chain;
    };
    let Some(index) = chain
        .iter()
        .position(|item| item.endpoint.kind == kind && item.endpoint.model == model)
    else {
        return chain;
    };
    let Some(picked) = chain.into_iter().nth(index) else {
        return Vec::new();
    };
    vec![picked]
}
