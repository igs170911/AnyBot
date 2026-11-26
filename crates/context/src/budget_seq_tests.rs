use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::input::StepRecord;
use crate::output::Compiled;

use crate::budget_tests::assemble_input;

#[test]
fn budget_trims_are_far_fewer_than_steps() {
    let mut events: Vec<anybot_protocol::TaskEvent> = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
    )];
    let mut trims: Vec<usize> = Vec::new();
    for index in 0..40usize {
        let steps: Vec<StepRecord> = (0..=index)
            .map(|i| sup::step_len(i, 1_500, &format!("r{i}")))
            .collect();
        let input = assemble_input(steps, 15_000, events.clone(), Vec::new());
        match compile(&input).unwrap() {
            Compiled::Sent { events: new, .. } => {
                let trimmed = new.iter().any(|e| match e {
                    crate::output::NewEvent::Collapsed { .. }
                    | crate::output::NewEvent::MemoryDropped { .. } => true,
                    crate::output::NewEvent::ContextBuilt => false,
                });
                if trimmed {
                    trims.push(index);
                }
                sup::push_new(&mut events, &new);
            }
            Compiled::Insufficient(_) => panic!("1500-char steps must fit at 15000 usable"),
        }
    }
    assert!(
        !trims.is_empty(),
        "budget trims happened trims={trims:?} events={}",
        events.len()
    );
    assert!(
        trims.len() <= 10,
        "trims far fewer than 40 steps: {trims:?}"
    );
    for pair in trims.windows(2) {
        let gap = pair
            .get(1)
            .unwrap_or(&0)
            .saturating_sub(*pair.first().unwrap_or(&0));
        assert!(
            gap >= 3,
            "three steps without trim after each trim: {trims:?}"
        );
    }
}

#[test]
fn each_sent_build_records_one_context_built() {
    let mut events: Vec<anybot_protocol::TaskEvent> = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
    )];
    let mut sent_builds = 0usize;
    for index in 0..60usize {
        let steps: Vec<StepRecord> = (0..=index)
            .map(|i| sup::step_len(i, 300, &format!("b{i}")))
            .collect();
        let input = assemble_input(steps, 31_000, events.clone(), Vec::new());
        match compile(&input).unwrap() {
            Compiled::Sent { events: new, .. } => {
                let built = new
                    .iter()
                    .filter(|e| matches!(e, crate::output::NewEvent::ContextBuilt))
                    .count();
                assert_eq!(built, 1, "one ContextBuilt per build");
                sent_builds = sent_builds.saturating_add(1);
                sup::push_new(&mut events, &new);
            }
            Compiled::Insufficient(_) => panic!("short steps must fit at 31000 usable"),
        }
    }
    let first = compile(&assemble_input(
        Vec::new(),
        31_000,
        vec![sup::known(
            1,
            anybot_protocol::KnownTaskEventKind::TaskCreated {
                goal: "g".to_owned(),
            },
        )],
        Vec::new(),
    ))
    .unwrap();
    assert!(
        matches!(first, Compiled::Sent { .. }),
        "opening build sends"
    );
    sent_builds = sent_builds.saturating_add(1);
    assert_eq!(sent_builds, 61, "60 steps plus the opening request");
    let built_total = events
        .iter()
        .filter(|e| {
            matches!(
                &e.kind,
                anybot_protocol::TaskEventKind::Known(
                    anybot_protocol::KnownTaskEventKind::ContextBuilt
                )
            )
        })
        .count();
    assert_eq!(
        built_total, 60,
        "one ContextBuilt event recorded before each step request"
    );
}
