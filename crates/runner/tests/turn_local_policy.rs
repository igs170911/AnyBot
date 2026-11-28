use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use anybot_protocol::{CardButton, CardKind, KnownTaskEventKind};
use anybot_trust::{ComputerUse, LocalPolicy, LockState, RunHere};
use runner_test::trust_world::{
    MAC, cards_of, first_turn, mac_setup, modes, never, param, rejected_reason,
};
use runner_test::turn::TurnWorld;
use runner_test::turn_helpers::{task_kinds, task_of};

#[tokio::test]
async fn t_safety_lock_5() -> Result<(), String> {
    let locked = LocalPolicy {
        direct: LockState::Unset,
        local_execution: LockState::LockedOff,
    };
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    world.trust.set_policy(MAC, locked);
    let outcome = first_turn(&world, "shell", "{\"line\":\"ls\"}").await?;
    let kinds = task_kinds(&world, &task_of(&outcome));
    let reason = rejected_reason(&kinds).ok_or("missing Rejected")?;
    assert_eq!(reason.code, "gate-local-locked");
    assert_eq!(param(&reason, "computer"), Some(MAC));
    assert!(cards_of(&kinds).is_empty());

    let other = TurnWorld::start(&mac_setup("Home Linux")).await?;
    other.trust.set_policy("Home Linux", locked);
    other.trust.set_use(
        "Home Linux",
        ComputerUse {
            usable_by_bot: false,
            run_here: RunHere::AlwaysAllow,
            this_machine: false,
        },
    );
    let second = first_turn(&other, "shell", "{\"line\":\"ls\"}").await?;
    let cards = cards_of(&task_kinds(&other, &task_of(&second)));
    let Some(KnownTaskEventKind::CardShown { kind, buttons, .. }) = cards.first() else {
        return Err("missing switch card".to_owned());
    };
    assert_eq!(*kind, CardKind::Switch);
    assert!(!buttons.contains(&CardButton::DirectMode));
    Ok(())
}

#[tokio::test]
async fn t_safety_local_3() -> Result<(), String> {
    let mut setup = mac_setup(MAC);
    setup.gate = modes(true, false);
    let world = TurnWorld::start(&setup).await?;
    world.trust.set_use(MAC, never());
    let outcome = first_turn(&world, "read_file", "{\"path\":\"/home/alice/work/a.md\"}").await?;
    let kinds = task_kinds(&world, &task_of(&outcome));
    let reason = rejected_reason(&kinds).ok_or("missing Rejected")?;
    assert_eq!(reason.code, "gate-local-never");
    assert_eq!(param(&reason, "computer"), Some(MAC));
    assert!(cards_of(&kinds).is_empty());
    Ok(())
}

#[tokio::test]
async fn t_autonomy_direct_3() -> Result<(), String> {
    let mut setup = mac_setup(MAC);
    setup.gate = modes(false, true);
    let world = TurnWorld::start(&setup).await?;
    world.trust.set_use(MAC, never());
    let outcome = first_turn(&world, "shell", "{\"line\":\"ls\"}").await?;
    let reason =
        rejected_reason(&task_kinds(&world, &task_of(&outcome))).ok_or("missing Rejected")?;
    assert_eq!(reason.code, "gate-local-never");
    Ok(())
}
