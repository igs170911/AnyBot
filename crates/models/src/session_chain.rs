use crate::session::{MAX_FALLBACKS, ModelRef};

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
    prefer_pending(chain, pending.or(adopted))
}

fn in_chain(chain: &[ModelRef], model: &ModelRef) -> bool {
    chain.iter().any(|item| same_endpoint(item, model))
}

fn same_endpoint(left: &ModelRef, right: &ModelRef) -> bool {
    left.endpoint.kind == right.endpoint.kind
        && left.endpoint.model == right.endpoint.model
        && left.endpoint.base_url == right.endpoint.base_url
}

fn prefer_pending(chain: Vec<ModelRef>, pending: Option<&ModelRef>) -> Vec<ModelRef> {
    let Some(pending) = pending else {
        return chain;
    };
    let Some(index) = chain.iter().position(|item| same_endpoint(item, pending)) else {
        return chain;
    };
    let mut ordered = chain;
    let picked = ordered.remove(index);
    ordered.insert(0, picked);
    ordered
}
