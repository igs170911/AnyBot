use std::sync::Arc;

use anybot_foundation::BotId;
use anybot_runner::{RunnerParts, TurnRunner};

use crate::turn::{TurnWorld, bot_profile, build_session, computer_of, turn_model};
use crate::turn_setup::share_clock;

pub fn second_bot_runner(world: &TurnWorld, name: &str) -> Result<TurnRunner, String> {
    let mut setup = world.setup.clone();
    name.clone_into(&mut setup.bot_name);
    let clock = share_clock(&world.clock);
    let bot = BotId::generate(clock.as_ref());
    let gated: Arc<dyn anybot_trust::GatedComputer> = world.runner_gated();
    Ok(TurnRunner::new(RunnerParts {
        gated,
        record: crate::mem_store::share_record(&world.record),
        raw: crate::turn_setup::share_raw(&world.raw),
        session: build_session(&world.clock)?,
        clock,
        bot: bot_profile(&setup, bot),
        model: turn_model(&setup, world.port),
        tools: setup.tools.clone(),
        lang: setup.lang,
        computer: computer_of(&setup),
        trust: Arc::clone(&world.trust) as Arc<dyn anybot_runner::TrustSettings>,
    }))
}
