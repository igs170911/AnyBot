use proptest::prelude::*;

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::step_record;
use crate::input::{BuildInput, PinnedSnapshot, StepRecord};
use crate::numbers::{
    K114_1_RECENT_STEPS, K114_2_DEFAULT_RESULT_CHARS, K114_3_ACCUMULATE_MARGIN_MILLI, milli_of,
};
use crate::output::{BlockKind, Compiled};

fn step_input(
    steps: Vec<StepRecord>,
    usable: u64,
    events: Vec<anybot_protocol::TaskEvent>,
) -> BuildInput {
    BuildInput {
        view_limit: K114_2_DEFAULT_RESULT_CHARS,
        events,
        user_turns: Vec::new(),
        bot_turns: Vec::new(),
        steps,
        constraints: Vec::new(),
        open_questions: Vec::new(),
        tool_specs: Vec::new(),
        system_prompt: "sys".to_owned(),
        pinned: PinnedSnapshot {
            version: "v1".to_owned(),
            items: Vec::new(),
        },
        memories: Vec::new(),
        prior_chat: Vec::new(),
        model: sup::plan(usable),
        compact_requested: false,
    }
}

#[test]
fn t_context_mask_1() {
    let usable: u64 = 32_000;
    let mut events: Vec<anybot_protocol::TaskEvent> = Vec::new();
    let mut final_request: Option<crate::output::CompiledRequest> = None;
    for index in 0..40usize {
        let steps: Vec<StepRecord> = (0..=index)
            .map(|i| sup::step_len(i, 700, &format!("r{i}")))
            .collect();
        let input = step_input(steps.clone(), usable, events.clone());
        if let Compiled::Sent {
            request,
            events: new,
            ..
        } = compile(&input).unwrap()
        {
            sup::push_new(&mut events, &new);
            final_request = Some(request);
        }
    }
    let request = final_request.expect("sent at every step");
    let masked: Vec<String> = request
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::ToolResultMasked)
        .flat_map(|b| b.source_ids.clone())
        .collect();
    let fulls: Vec<String> = request
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::ToolResultFull)
        .flat_map(|b| b.source_ids.clone())
        .collect();
    let recent: Vec<String> = (0..40usize)
        .map(|i| format!("result-r{i}"))
        .rev()
        .take(K114_1_RECENT_STEPS)
        .collect();
    for id in &recent {
        assert!(
            fulls.contains(id),
            "recent ten verbatim: fulls={fulls:?} masked={masked:?}"
        );
    }
    for id in &masked {
        assert!(!recent.contains(id), "only old results masked");
    }
    assert!(!masked.is_empty(), "masking happened");
    let in_request = |id: &String| id.as_str().starts_with("result-");
    assert_eq!(
        masked.iter().filter(|i| in_request(i)).count()
            + fulls.iter().filter(|i| in_request(i)).count(),
        40,
        "every step result is present in the request"
    );
    let collapsed_count = events
        .iter()
        .filter(|e| (anybot_domain::parse_collapsed(&e.kind).unwrap()).is_some())
        .count();
    assert!(collapsed_count >= 1);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(150))]
    #[test]
    fn accumulate_margin_between_collapse_events(
        lens in prop::collection::vec(step_record(0).prop_map(|s| s.content.chars().count()), 12..22),
        usable in 4_000u64..40_000u64,
    ) {
        let mut events: Vec<anybot_protocol::TaskEvent> = Vec::new();
        let mut last_ids: Vec<String> = Vec::new();
        let mut pending: u64 = 0;
        let floor = milli_of(usable, K114_3_ACCUMULATE_MARGIN_MILLI);
        for (index, len) in lens.iter().enumerate() {
            let steps: Vec<StepRecord> = (0..=index)
                .map(|i| sup::step_len(i, *len, &format!("r{i}")))
                .collect();
            let input = step_input(steps.clone(), usable, events.clone());
            if let Compiled::Sent { events: new, .. } = compile(&input).unwrap() {
                sup::push_new(&mut events, &new);
                let fresh = events.iter().rev().find_map(|e| anybot_domain::parse_collapsed(&e.kind).unwrap());
                if let Some(c) = fresh.filter(|c| c.reason == anybot_domain::ContextCollapseReason::Accumulated) {
                        let added: usize = c
                            .ids
                            .iter()
                            .filter(|id| !last_ids.iter().any(|x| x.as_str() == id.as_str()))
                            .count();
                        let added_tokens: u64 = steps
                            .iter()
                            .filter(|s| c.ids.contains(&s.source_ref))
                            .map(|s| u64::try_from(s.content.chars().count()).unwrap_or(0))
                            .sum();
                        pending = pending.saturating_add(added_tokens);
                        if added > 0 {
                            prop_assert!(pending >= floor, "accumulated before collapsing");
                            pending = 0;
                        }
                    last_ids.clone_from(&c.ids);
                }
            }
        }
    }
}
