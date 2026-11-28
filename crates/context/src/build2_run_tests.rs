use proptest::prelude::*;

use crate::build_tests::{
    collapse_recorded, model_swap_present, prefix_of, push_built, rebuilt_usable, step_at,
};
use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::event_with_seq;
use crate::input::{BuildInput, StepRecord};
use crate::output::{Block, Compiled, NewEvent};

struct Runner {
    base: BuildInput,
    events: Vec<anybot_protocol::TaskEvent>,
    prev: Option<Vec<Block>>,
    version: u64,
}

fn swap_event(events: &mut Vec<anybot_protocol::TaskEvent>) {
    let seq = u64::try_from(events.len()).unwrap_or(0).saturating_add(1);
    events.push(event_with_seq(
        seq,
        anybot_protocol::TaskEventKind::Known(anybot_protocol::KnownTaskEventKind::ModelSwapped {
            from: "a".to_owned(),
            to: "b".to_owned(),
            reason: "r".to_owned(),
        }),
    ));
}

fn prefix_check(
    run: &Runner,
    current: &[Block],
    index: usize,
    fresh_collapsed: bool,
    swap_here: bool,
    update_here: bool,
) -> Result<(), TestCaseError> {
    let Some(prev_prefix) = &run.prev else {
        return Ok(());
    };
    if fresh_collapsed || swap_here || update_here {
        return Ok(());
    }
    let slice = current.get(..prev_prefix.len()).unwrap_or(&[]);
    let prev_bytes = serde_json::to_vec(prev_prefix).unwrap_or_default();
    let cur_bytes = serde_json::to_vec(slice).unwrap_or_default();
    prop_assert!(
        current.len() >= prev_prefix.len() && prev_bytes == cur_bytes,
        "prefix bytes differed with no exception at step {index}",
    );
    Ok(())
}

fn step(
    run: &mut Runner,
    index: usize,
    lens: &[usize],
    n: usize,
    swap_at: usize,
    update_at: usize,
    mode: u8,
) -> Result<(), TestCaseError> {
    let do_swap = mode == 2 || mode == 3;
    let do_update = mode == 1 || mode == 3;
    let mut swap_here = false;
    let mut update_here = false;
    if do_swap && Some(index) == swap_at.checked_rem(n) {
        swap_event(&mut run.events);
        swap_here = true;
    }
    if do_update && Some(index) == update_at.checked_rem(n) {
        run.version = run.version.saturating_add(1);
        update_here = true;
    }
    let many: Vec<StepRecord> = (0..=index)
        .map(|i| step_at(i, *lens.get(i).unwrap_or(&1_500), &format!("r{i}")))
        .collect();
    let usable = if mode == 3 { 4_000 } else { 100_000 };
    let input = rebuilt_usable(
        &run.base,
        many,
        run.events.clone(),
        &run.version.to_string(),
        usable,
    );
    let compiled = compile(&input).map_err(|e| TestCaseError::fail(format!("{e:?}")))?;
    if let Compiled::Sent {
        request,
        events: new,
        ..
    } = &compiled
    {
        let fresh_collapsed = new.iter().any(|e| matches!(e, NewEvent::Collapsed { .. }));
        sup::push_new(&mut run.events, new);
        let current = prefix_of(request);
        prefix_check(
            run,
            &current,
            index,
            fresh_collapsed,
            swap_here,
            update_here,
        )?;
        run.prev = Some(current);
    } else {
        push_built(&mut run.events);
        run.prev = None;
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_build_2(
        lens in prop::collection::vec(1_500usize..3_000usize, 11..16),
        swap_at in 0usize..12,
        update_at in 0usize..12,
        mode in 0u8..4u8,
    ) {
        let mut run = Runner {
            base: sup::base_input(),
            events: Vec::new(),
            prev: None,
            version: 1,
        };
        let n = lens.len();
        for index in 0..n {
            step(&mut run, index, &lens, n, swap_at, update_at, mode)?;
        }
        if mode == 1 || mode == 3 {
            prop_assert!(run.version > 1, "snapshot update must be generated");
        }
        if mode == 2 || mode == 3 {
            prop_assert!(model_swap_present(&run.events), "swap must be generated");
        }
        if mode == 3 {
            prop_assert!(collapse_recorded(&run.events), "collapse must be generated");
        }
    }
}
