use anybot_protocol::ComputerOwner;
use anybot_trust::{
    ActionRequest, CallContext, GateConfig, GateStep, LockState, StepFour, SwitchInfo,
    SwitchReason, computer_use_step, direct_button_offered, step_denied_run, step_four_denial,
    switch_ticket,
};

use crate::error::TurnError;
use crate::runner::TurnRunner;
use crate::view::ActionView;

struct Loaded {
    config: GateConfig,
    four: StepFour,
    direct_button: bool,
}

fn load(runner: &TurnRunner, use_step_four: bool) -> Result<Loaded, TurnError> {
    let computer = &*runner.gated;
    let mut config = computer.gate_config();
    if config.bot_name.is_empty() {
        config.bot_name.clone_from(&runner.bot.name);
    }
    let settings = &*runner.trust;
    let policy = settings.policy(computer.name())?;
    config.modes = settings.modes()?;
    config.modes.mdm_locks_direct =
        config.modes.mdm_locks_direct || policy.direct == LockState::LockedOff;
    config.user = settings.user_rules(&runner.bot.id, runner.conversation())?;
    let four = if !use_step_four || computer.declared_caps().owner == ComputerOwner::Bot {
        StepFour::Pass
    } else {
        let usage = settings.computer_use(&runner.bot.id, computer.name())?;
        computer_use_step(&usage, &policy, &config.modes)
    };
    let direct_button = direct_button_offered(&config.modes, &policy);
    Ok(Loaded {
        config,
        four,
        direct_button,
    })
}

pub(crate) fn decide(
    runner: &TurnRunner,
    request: &ActionRequest,
    call_ctx: &CallContext,
    view: &ActionView,
    use_step_four: bool,
) -> Result<GateStep, TurnError> {
    let loaded = load(runner, use_step_four)?;
    let computer = &*runner.gated;
    if let Some(denial) = step_four_denial(&loaded.four, computer.name()) {
        return Ok(GateStep::Denied(step_denied_run(
            request,
            &denial,
            &*runner.clock,
        )));
    }
    let step = anybot_trust::gate_step(
        request,
        call_ctx,
        computer,
        &loaded.config,
        view,
        &*runner.clock,
    );
    match (loaded.four, step) {
        (StepFour::Switch { reasons }, step) if wants_switch_card(&reasons, &step) => {
            let info = SwitchInfo {
                bot: runner.bot.name.clone(),
                reasons,
                direct_button: loaded.direct_button,
            };
            Ok(GateStep::Ask(Box::new(switch_ticket(
                request, call_ctx, computer, info,
            ))))
        }
        (_, other) => Ok(other),
    }
}

fn wants_switch_card(reasons: &[SwitchReason], step: &GateStep) -> bool {
    match step {
        GateStep::Allow(_) => true,
        GateStep::Ask(_) => reasons.contains(&SwitchReason::BotNotAllowed),
        GateStep::Denied(_) => false,
    }
}
