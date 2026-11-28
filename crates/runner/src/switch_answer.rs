use crate::action::OneAction;
use crate::approvals::PendingCard;
use crate::error::TurnError;
use crate::runner::{TurnCtx, TurnRunner};

pub(crate) async fn continue_after_switch(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    card: &PendingCard,
) -> Result<OneAction, TurnError> {
    let request = &card.request;
    let json = crate::action::args_json(&request.arguments)?;
    let Ok(args) = anybot_tools::parse_call(&request.tool, &json) else {
        return Ok(OneAction::Done);
    };
    let (areq, view) = crate::action::build_areq(runner, ctx, request, args)?;
    let call_ctx =
        anybot_trust::CallContext::new(request.call_id, ctx.turn_id, &request.provider_call_id);
    let step = crate::gate_run::decide(runner, &areq, &call_ctx, &view, false)?;
    Box::pin(crate::action::apply_step(
        runner, ctx, request, &areq, view, step,
    ))
    .await
}
