use anybot_context::Compiled;
use anybot_models::{
    AskOutcome, Audience, Calibration, FailKind, GatewayFail, ModelChoice, ModelRef,
    ProviderEndpoint, ThinkingMode, TurnRequest, ask_plan, model_key, resolve,
};
use anybot_protocol::{KnownTaskEventKind, ModelName, TurnEndReason};

use crate::answer::model_name_of;
use crate::context_input::build_input_for;
use crate::ending::end_reason;
use crate::error::TurnError;
use crate::halt::finish_halt;
use crate::record::Appended;
use crate::runner::{TurnCtx, TurnModel, TurnRunner};
use crate::step::{PreparedSent, StepFlow, prepare_sent};

pub trait ModelReach: Send + Sync {
    fn any_online_computer_reaches(&self, model: &ModelName) -> bool;
}

pub struct NoOnlineModelReach;

impl ModelReach for NoOnlineModelReach {
    fn any_online_computer_reaches(&self, _model: &ModelName) -> bool {
        false
    }
}

impl TurnModel {
    #[must_use]
    pub fn copy_model(&self) -> Self {
        Self {
            primary: self.primary.copy_ref(),
            fallbacks: self.fallbacks.iter().map(ModelRef::copy_ref).collect(),
            thinking: self.thinking,
            capability: self.capability.clone(),
            calibration: self.calibration.clone(),
            reserved_output: self.reserved_output,
        }
    }
}

pub(crate) enum FallbackResult {
    Answered {
        outcome: Box<AskOutcome>,
        manifest_id: String,
    },
    Ended(StepFlow),
}

enum StepAttempt {
    Answered {
        outcome: Box<AskOutcome>,
        manifest_id: String,
    },
    Failed {
        next_model: Box<TurnModel>,
        next_fail: GatewayFail,
    },
    HaltedOrEnded(StepFlow),
}

pub(crate) async fn ask_step_with(
    runner: &TurnRunner,
    model: &TurnModel,
    request: &TurnRequest,
) -> Result<AskOutcome, GatewayFail> {
    let mut session = runner.session.lock().await;
    let mut plan = ask_plan(
        &model.primary,
        &model.fallbacks,
        request,
        Audience::Watching,
    );
    plan.bot = &runner.bot.name;
    plan.stream = true;
    session.ask_step(&mut plan).await
}

pub(crate) async fn handle_ask_fail(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    starting: &TurnModel,
    initial: GatewayFail,
) -> Result<FallbackResult, TurnError> {
    let mut current = starting.copy_model();
    let mut fail = initial;
    loop {
        if let Some(halt) = crate::halt::check_halt(runner, ctx).await? {
            let reason = finish_halt(runner, ctx, halt).await?;
            return Ok(FallbackResult::Ended(StepFlow::Ended(reason)));
        }
        match fail {
            GatewayFail::Exhausted {
                fallback: Some(choice),
                ..
            } => match try_fallback_step(runner, ctx, &current, &choice).await? {
                StepAttempt::Answered {
                    outcome,
                    manifest_id,
                } => {
                    return Ok(FallbackResult::Answered {
                        outcome,
                        manifest_id,
                    });
                }
                StepAttempt::HaltedOrEnded(flow) => return Ok(FallbackResult::Ended(flow)),
                StepAttempt::Failed {
                    next_model,
                    next_fail,
                } => {
                    current = *next_model;
                    fail = next_fail;
                }
            },
            other @ (GatewayFail::Exhausted { fallback: None, .. }
            | GatewayFail::MissingKey
            | GatewayFail::KeyDestination
            | GatewayFail::Status(_)
            | GatewayFail::Transport(_)
            | GatewayFail::Stopped
            | GatewayFail::Classified { .. }) => {
                let flow = finish_terminal_fail(runner, ctx, &current, &other).await?;
                return Ok(FallbackResult::Ended(flow));
            }
        }
    }
}

async fn try_fallback_step(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    current: &TurnModel,
    choice: &ModelChoice,
) -> Result<StepAttempt, TurnError> {
    let next = build_next_model(runner, current, choice).await;
    let swap = KnownTaskEventKind::ModelSwapped {
        from: current.primary.endpoint.model.clone(),
        to: next.primary.endpoint.model.clone(),
        reason: choice.reason_code.to_owned(),
    };
    if let Appended::Halted(halt) = runner.append_turn_unless_halted(ctx, swap).await? {
        let reason = finish_halt(runner, ctx, halt).await?;
        return Ok(StepAttempt::HaltedOrEnded(StepFlow::Ended(reason)));
    }
    ctx.set_active_model(next.copy_model());
    let input = build_input_for(runner, ctx, &next).await?;
    let compiled = anybot_context::compile(&input).map_err(TurnError::Compile)?;
    let (request, manifest, events) = match compiled {
        Compiled::Insufficient(short) => {
            let reason = TurnEndReason::ContextBudget {
                deficit_tokens: Some(short.deficit_tokens),
            };
            runner.append_turn_end(ctx, reason.clone(), false).await?;
            return Ok(StepAttempt::HaltedOrEnded(StepFlow::Ended(reason)));
        }
        Compiled::Sent {
            request,
            manifest,
            events,
        } => (request, manifest, events),
    };
    let (manifest_id, turn_request) =
        match prepare_sent(runner, ctx, &next, &request, &manifest, &events).await? {
            PreparedSent::Ready {
                manifest_id,
                turn_request,
            } => (manifest_id, turn_request),
            PreparedSent::Halted(reason) => {
                return Ok(StepAttempt::HaltedOrEnded(StepFlow::Ended(reason)));
            }
        };
    match ask_step_with(runner, &next, &turn_request).await {
        Ok(outcome) => Ok(StepAttempt::Answered {
            outcome: Box::new(outcome),
            manifest_id,
        }),
        Err(next_fail) => Ok(StepAttempt::Failed {
            next_model: Box::new(next),
            next_fail,
        }),
    }
}

async fn finish_terminal_fail(
    runner: &TurnRunner,
    ctx: &TurnCtx,
    current: &TurnModel,
    fail: &GatewayFail,
) -> Result<StepFlow, TurnError> {
    if is_unreachable_fail(fail)
        && !runner
            .model_reach
            .any_online_computer_reaches(&model_name_of(&current.primary.endpoint))
    {
        if let Appended::Halted(halt) = runner
            .append_turn_unless_halted(ctx, KnownTaskEventKind::ModelUnreachable)
            .await?
        {
            let reason = finish_halt(runner, ctx, halt).await?;
            return Ok(StepFlow::Ended(reason));
        }
        let reason = TurnEndReason::ModelUnreachable;
        runner.append_turn_end(ctx, reason.clone(), false).await?;
        return Ok(StepFlow::Ended(reason));
    }
    let reason = end_reason(fail);
    runner.append_turn_end(ctx, reason.clone(), false).await?;
    Ok(StepFlow::Ended(reason))
}

fn is_unreachable_fail(fail: &GatewayFail) -> bool {
    match fail {
        GatewayFail::Exhausted { classified, .. } | GatewayFail::Classified { classified, .. } => {
            classified.kind == FailKind::Unreachable
        }
        GatewayFail::MissingKey
        | GatewayFail::KeyDestination
        | GatewayFail::Status(_)
        | GatewayFail::Transport(_)
        | GatewayFail::Stopped => false,
    }
}

async fn build_next_model(
    runner: &TurnRunner,
    current: &TurnModel,
    choice: &ModelChoice,
) -> TurnModel {
    let endpoint = &choice.model.endpoint;
    let key = model_key(endpoint.kind, &endpoint.base_url, &endpoint.model);
    let session = runner.session.lock().await;
    let learned = session.learned().iter().find(|row| row.key == key).cloned();
    drop(session);
    let capability = resolve(
        endpoint.kind,
        &endpoint.model,
        choice.model.bot_context,
        learned.as_ref(),
    );
    let calibration = learned
        .as_ref()
        .map_or_else(Calibration::new, |row| row.calibration.clone());
    TurnModel {
        primary: choice.model.copy_ref(),
        fallbacks: remaining_after(&current.fallbacks, endpoint),
        thinking: ThinkingMode::Auto,
        capability,
        calibration,
        reserved_output: current.reserved_output,
    }
}

fn remaining_after(fallbacks: &[ModelRef], chosen: &ProviderEndpoint) -> Vec<ModelRef> {
    fallbacks
        .iter()
        .skip_while(|item| !same_endpoint(&item.endpoint, chosen))
        .skip(1)
        .map(ModelRef::copy_ref)
        .collect()
}

fn same_endpoint(left: &ProviderEndpoint, right: &ProviderEndpoint) -> bool {
    left.kind == right.kind && left.model == right.model && left.base_url == right.base_url
}
