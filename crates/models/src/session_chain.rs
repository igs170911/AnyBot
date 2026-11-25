use crate::session::{MAX_FALLBACKS, ModelRef};
use crate::types::ProviderKind;

pub(crate) fn chain_of<'a>(
    primary: &'a ModelRef,
    fallbacks: &'a [ModelRef],
    pending: Option<&(ProviderKind, String)>,
) -> Vec<&'a ModelRef> {
    let mut chain = vec![primary];
    for item in fallbacks.iter().take(MAX_FALLBACKS) {
        chain.push(item);
    }
    prefer_pending(chain, pending)
}

fn prefer_pending<'a>(
    chain: Vec<&'a ModelRef>,
    pending: Option<&(ProviderKind, String)>,
) -> Vec<&'a ModelRef> {
    let Some((kind, model)) = pending else {
        return chain;
    };
    let Some(picked) = chain
        .iter()
        .copied()
        .find(|item| item.endpoint.kind == *kind && item.endpoint.model == *model)
    else {
        return chain;
    };
    vec![picked]
}
