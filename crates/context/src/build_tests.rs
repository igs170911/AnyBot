use proptest::prelude::*;

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::{build_input, event_with_seq, events_strategy, mixed_text};
use crate::input::{BuildInput, ChatRecord, PinnedSnapshot, StepRecord};
use crate::numbers::{K114_5_REF_HEAD_CHARS, K114_5_USER_REF_MILLI, milli_of};
use crate::output::{Block, BlockForm, BlockKind, Compiled, CompiledRequest, Manifest};
use crate::state::build_state;

pub(crate) fn step_at(index: usize, len: usize, ref_id: &str) -> StepRecord {
    sup::step_len(index, len, ref_id)
}

pub(crate) fn rebuilt(
    base: &BuildInput,
    steps: Vec<StepRecord>,
    events: Vec<anybot_protocol::TaskEvent>,
    version: &str,
) -> BuildInput {
    BuildInput {
        view_limit: base.view_limit,
        events,
        user_turns: base.user_turns.clone(),
        bot_turns: base.bot_turns.clone(),
        steps,
        constraints: base.constraints.clone(),
        open_questions: base.open_questions.clone(),
        tool_specs: base.tool_specs.clone(),
        system_prompt: base.system_prompt.clone(),
        pinned: PinnedSnapshot {
            version: version.to_owned(),
            items: vec!["pin".to_owned()],
        },
        memories: base.memories.clone(),
        prior_chat: base.prior_chat.clone(),
        model: base.model.clone(),
        compact_requested: base.compact_requested,
    }
}

pub(crate) fn prefix_of(request: &CompiledRequest) -> Vec<Block> {
    request
        .blocks
        .iter()
        .take(request.prefix_len)
        .cloned()
        .collect()
}

pub(crate) fn collapse_recorded(events: &[anybot_protocol::TaskEvent]) -> bool {
    events.iter().any(|e| {
        (anybot_domain::parse_collapsed(&e.kind).unwrap()).is_some_and(|c| !c.ids.is_empty())
    })
}

pub(crate) fn model_swap_present(events: &[anybot_protocol::TaskEvent]) -> bool {
    events.iter().any(|e| {
        matches!(
            &e.kind,
            anybot_protocol::TaskEventKind::Known(
                anybot_protocol::KnownTaskEventKind::ModelSwapped { .. }
            )
        )
    })
}

pub(crate) fn push_built(events: &mut Vec<anybot_protocol::TaskEvent>) {
    let seq = u64::try_from(events.len()).unwrap_or(0).saturating_add(1);
    events.push(event_with_seq(
        seq,
        anybot_protocol::TaskEventKind::Known(sup::context_built_kind()),
    ));
}

pub(crate) fn rebuilt_usable(
    base: &BuildInput,
    steps: Vec<StepRecord>,
    events: Vec<anybot_protocol::TaskEvent>,
    version: &str,
    usable: u64,
) -> BuildInput {
    let mut out = rebuilt(base, steps, events, version);
    out.model.capability.context_tokens = usable.saturating_add(out.model.reserved_output);
    out
}

fn has(req: &CompiledRequest, kind: BlockKind, pred: impl Fn(&Block) -> bool) -> bool {
    req.blocks.iter().any(|b| b.kind == kind && pred(b))
}

fn user_turns_must_ok(
    req: &CompiledRequest,
    manifest: &Manifest,
    turns: &[ChatRecord],
    usable: u64,
) -> bool {
    let threshold = milli_of(usable, K114_5_USER_REF_MILLI);
    turns.iter().all(|u| {
        let tokens = anybot_models::formula_tokens(&anybot_models::EstimateInput {
            text: &u.text,
            messages: 1,
            images: 0,
        });
        if tokens <= threshold {
            return has(req, BlockKind::UserText, |b| {
                b.form == BlockForm::Verbatim && b.text == u.text
            });
        }
        let head: String = u.text.chars().take(K114_5_REF_HEAD_CHARS).collect();
        let char_len = u.text.chars().count();
        has(req, BlockKind::UserRef, |b| {
            b.form == BlockForm::Reference
                && b.source_ids.contains(&u.record_id)
                && b.text.contains(&u.record_id)
                && b.text.contains(&format!("len {char_len}"))
                && b.text.contains(&head)
        }) && manifest.included.iter().any(|e| {
            e.kind == BlockKind::UserRef
                && e.form == BlockForm::Reference
                && e.source_ids.contains(&u.record_id)
        })
    })
}

fn has_must_tail(
    input: &BuildInput,
    req: &CompiledRequest,
    st: &crate::state::StateBuilder,
) -> bool {
    let step_ok = st
        .steps
        .iter()
        .filter(|(_, s)| s.state == crate::state::StepStateKind::Open)
        .max_by_key(|(_, s)| s.intent_seq)
        .is_none_or(|(id, _)| {
            has(req, BlockKind::CurrentStep, |b| {
                b.form == BlockForm::Verbatim && b.source_ids.contains(id)
            })
        });
    let q_ok = input.open_questions.iter().all(|q| {
        has(req, BlockKind::OpenQuestion, |b| {
            b.form == BlockForm::Verbatim && b.text == *q
        })
    }) && st.open_cards().iter().all(|(seq, action)| {
        has(req, BlockKind::OpenQuestion, |b| {
            b.form == BlockForm::Verbatim
                && b.source_ids.contains(&format!("question-{seq}"))
                && b.text.contains(action)
        })
    });
    let pending_ok = st.pending.keys().chain(st.unknowns.keys()).all(|id| {
        has(req, BlockKind::PendingAction, |b| {
            b.form == BlockForm::Verbatim && b.source_ids.contains(id)
        })
    }) && input.steps.iter().filter(|s| s.keeps_full()).all(|s| {
        has(req, BlockKind::PendingAction, |b| {
            b.form == BlockForm::Verbatim && b.source_ids.contains(&s.call_id.as_str())
        })
    });
    step_ok && q_ok && pending_ok
}

fn must_items_ok(input: &BuildInput, req: &CompiledRequest, manifest: &Manifest) -> bool {
    let st = build_state(&input.events);
    let prefix_ok = has(req, BlockKind::System, |b| {
        b.form == BlockForm::Verbatim && b.text == input.system_prompt
    }) && input.tool_specs.iter().all(|s| {
        has(req, BlockKind::ToolDoc, |b| {
            b.form == BlockForm::Verbatim
                && b.text.contains(&s.name)
                && b.text.contains(&s.description)
        })
    }) && input.constraints.iter().all(|c| {
        has(req, BlockKind::Constraint, |b| {
            b.form == BlockForm::Verbatim && b.text == *c
        })
    }) && st.criteria.iter().all(|c| {
        has(req, BlockKind::Criterion, |b| {
            b.form == BlockForm::Verbatim && b.text == c.text
        })
    });
    prefix_ok
        && has_must_tail(input, req, &st)
        && user_turns_must_ok(req, manifest, &input.user_turns, input.model.usable())
}

fn must_strategy() -> BoxedStrategy<BuildInput> {
    (
        prop::collection::vec(mixed_text(30), 1..30),
        prop::collection::vec((mixed_text(20), 0usize..20_000usize), 1..4),
        prop::collection::vec((mixed_text(20), 0usize..20_000usize), 0..2),
        events_strategy(6),
        2_000u64..=200_000u64,
        build_input(2_000, 200_000, 4),
    )
        .prop_map(
            |(constraints, users, criteria, mut events, usable, mut base)| {
                let mut seq = u64::try_from(events.len()).unwrap_or(0).saturating_add(10);
                let crits: Vec<anybot_protocol::Criterion> = criteria
                    .into_iter()
                    .map(|(head, pad)| anybot_protocol::Criterion::ModelJudgement {
                        text: format!("{head}{}", "j".repeat(pad)),
                    })
                    .collect();
                if !crits.is_empty() {
                    events.push(sup::done_when_event(seq, crits));
                }
                base.events = events;
                base.constraints = constraints;
                base.user_turns = users
                    .into_iter()
                    .enumerate()
                    .map(|(i, (head, pad))| {
                        seq = seq.saturating_add(1);
                        ChatRecord {
                            record_id: format!("mu-{i}"),
                            text: format!("{head}{}", "u".repeat(pad)),
                            seq,
                        }
                    })
                    .collect();
                base.model.capability.context_tokens =
                    usable.saturating_add(base.model.reserved_output);
                base
            },
        )
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_build_1(input in build_input(1_000, 100_000, 8)) {
        let first = compile(&input).unwrap();
        let second = compile(&input).unwrap();
        let a = match &first {
            Compiled::Sent { request, .. } => serde_json::to_vec(request).unwrap(),
            Compiled::Insufficient(x) => serde_json::to_vec(x).unwrap(),
        };
        let b = match &second {
            Compiled::Sent { request, .. } => serde_json::to_vec(request).unwrap(),
            Compiled::Insufficient(x) => serde_json::to_vec(x).unwrap(),
        };
        prop_assert_eq!(a, b);
    }

    #[test]
    fn t_context_must_1(input in must_strategy()) {
        match compile(&input).unwrap() {
            Compiled::Sent { request, manifest, .. } => {
                prop_assert!(must_items_ok(&input, &request, &manifest));
            }
            Compiled::Insufficient(x) => {
                prop_assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
            }
        }
    }

    #[test]
    fn t_context_budget_1(input in build_input(4_000, 1_000_000, 6)) {
        match compile(&input).unwrap() {
            Compiled::Sent { request, .. } => {
                let usable = input.model.usable();
                let own_raw: u64 = request
                    .blocks
                    .iter()
                    .map(|b| {
                        anybot_models::formula_tokens(&anybot_models::EstimateInput {
                            text: &b.text,
                            messages: 1,
                            images: 0,
                        })
                    })
                    .sum();
                let own_cal = anybot_models::apply_calibration(own_raw, &input.model.calibration);
                prop_assert!(
                    own_cal <= usable,
                    "estimate with margin {own_cal} over usable {usable}"
                );
            }
            Compiled::Insufficient(x) => {
                prop_assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
            }
        }
    }
}
