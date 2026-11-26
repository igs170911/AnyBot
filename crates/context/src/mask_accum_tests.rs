use proptest::prelude::*;

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::unique_step_refs;
use crate::input::{BuildInput, ChatRecord, PinnedSnapshot, StepRecord};
use crate::mask_prop_tests::mask_2_problems;
use crate::numbers::{
    K114_2_DEFAULT_RESULT_CHARS, K114_3_ACCUMULATE_MARGIN_MILLI, K114_3_SHORT_RESULT_CHARS,
    milli_of,
};
use crate::output::{BlockKind, Compiled, CompiledRequest};

pub(crate) const FIXED_LENS: [usize; 4] = [300, 600, 900, 2_500];

fn mk_step(index: usize, len: usize) -> StepRecord {
    let levels: [anybot_protocol::ToolEffect; 3] = [
        anybot_protocol::ToolEffect::ReadOnly,
        anybot_protocol::ToolEffect::Repeatable,
        anybot_protocol::ToolEffect::Irreversible,
    ];
    let outcomes: [crate::input::StepOutcomeKind; 7] = [
        crate::input::StepOutcomeKind::Completed,
        crate::input::StepOutcomeKind::Failed,
        crate::input::StepOutcomeKind::Unknown,
        crate::input::StepOutcomeKind::Pending,
        crate::input::StepOutcomeKind::Completed,
        crate::input::StepOutcomeKind::Completed,
        crate::input::StepOutcomeKind::Completed,
    ];
    let head = format!("s{index} ");
    let pad = len.saturating_sub(head.chars().count());
    StepRecord {
        seq: u64::try_from(index).unwrap_or(0).saturating_add(1),
        call_id: anybot_foundation::CallId::parse(&format!("00000000-0000-0000-0000-{index:012x}"))
            .unwrap(),
        tool: format!("tool{index}"),
        args_summary: format!("a{index}"),
        level: *levels
            .get(index % 3)
            .unwrap_or(&anybot_protocol::ToolEffect::ReadOnly),
        unconfirmed: index.is_multiple_of(3),
        outcome: *outcomes
            .get(index % 7)
            .unwrap_or(&crate::input::StepOutcomeKind::Completed),
        source_ref: format!("ref{index}"),
        content: format!("{head}{}", "x".repeat(pad)),
        exit_code: if outcomes.get(index % 7).copied()
            == Some(crate::input::StepOutcomeKind::Failed)
        {
            Some(1)
        } else {
            None
        },
        error_at: Vec::new(),
    }
}

fn interleave(
    users: Vec<ChatRecord>,
    bots: Vec<ChatRecord>,
    steps: Vec<StepRecord>,
) -> (Vec<ChatRecord>, Vec<ChatRecord>, Vec<StepRecord>) {
    let mut seq = 1u64;
    let mut users = users;
    let mut bots = bots;
    let mut steps = steps;
    let width = users.len().max(bots.len()).max(steps.len());
    for i in 0..width {
        if let Some(u) = users.get_mut(i) {
            u.seq = seq;
            seq = seq.saturating_add(1);
        }
        if let Some(b) = bots.get_mut(i) {
            b.seq = seq;
            seq = seq.saturating_add(1);
        }
        if let Some(s) = steps.get_mut(i) {
            s.seq = seq;
            seq = seq.saturating_add(1);
        }
    }
    (users, bots, steps)
}

fn seq_for_block(block: &crate::output::Block, steps: &[StepRecord]) -> Option<u64> {
    for id in &block.source_ids {
        if let Some(step) = steps.iter().find(|s| {
            id == &format!("call-{}", s.call_id.as_str())
                || id == &format!("result-{}", s.source_ref)
        }) {
            return Some(step.seq);
        }
    }
    None
}

fn order_problems(request: &CompiledRequest, steps: &[StepRecord]) -> Vec<String> {
    let start = request
        .blocks
        .iter()
        .take_while(|b| b.kind != BlockKind::ToolIntent)
        .count();
    let end = request
        .blocks
        .iter()
        .take_while(|b| !matches!(b.kind, BlockKind::StateSummary | BlockKind::Memory))
        .count();
    let mut problems = Vec::new();
    let mut prev = 0u64;
    for b in request.blocks.iter().take(end).skip(start) {
        if let Some(seq) = seq_for_block(b, steps) {
            if seq < prev {
                problems.push(format!("segment order broken: {seq} after {prev}"));
            }
            prev = seq;
        }
    }
    problems
}

fn mask_input(
    users: &[ChatRecord],
    bots: &[ChatRecord],
    steps: &[StepRecord],
    usable: u64,
    events: Vec<anybot_protocol::TaskEvent>,
) -> BuildInput {
    BuildInput {
        view_limit: K114_2_DEFAULT_RESULT_CHARS,
        events,
        user_turns: users.to_vec(),
        bot_turns: bots.to_vec(),
        steps: steps.to_vec(),
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

#[derive(Default)]
pub(crate) struct Occurrences {
    pub(crate) cases: usize,
    pub(crate) sent_cases: usize,
    pub(crate) accumulate_collapses: usize,
    pub(crate) margin_checks: usize,
    pub(crate) short_kept: usize,
    pub(crate) masks_seen: usize,
}

struct CaseState {
    events: Vec<anybot_protocol::TaskEvent>,
    last_ids: Vec<String>,
    pending: u64,
    floor: u64,
}

fn used_upto(
    steps: &[StepRecord],
    users: &[ChatRecord],
    bots: &[ChatRecord],
    index: usize,
) -> (Vec<StepRecord>, Vec<ChatRecord>, Vec<ChatRecord>) {
    let used_steps: Vec<StepRecord> = steps
        .iter()
        .take(index.saturating_add(1))
        .cloned()
        .collect();
    let last_seq = used_steps.last().map_or(0, |s| s.seq);
    let used_users: Vec<ChatRecord> = users
        .iter()
        .filter(|u| u.seq <= last_seq)
        .cloned()
        .collect();
    let used_bots: Vec<ChatRecord> = bots.iter().filter(|b| b.seq <= last_seq).cloned().collect();
    (used_steps, used_users, used_bots)
}

fn margin_round(
    c: &anybot_domain::ContextCollapsed,
    used_steps: &[StepRecord],
    state: &mut CaseState,
    st: &mut Occurrences,
) -> Result<(), TestCaseError> {
    let added: usize = c
        .ids
        .iter()
        .filter(|id| !state.last_ids.iter().any(|x| x.as_str() == id.as_str()))
        .count();
    let added_tokens: u64 = used_steps
        .iter()
        .filter(|s| c.ids.contains(&s.source_ref))
        .map(|s| u64::try_from(s.content.chars().count()).unwrap_or(0))
        .sum();
    state.pending = state.pending.saturating_add(added_tokens);
    if added > 0 {
        prop_assert!(
            state.pending >= state.floor,
            "collapsed before accumulating the margin",
        );
        state.pending = 0;
        st.margin_checks = st.margin_checks.saturating_add(1);
    }
    st.accumulate_collapses = st.accumulate_collapses.saturating_add(1);
    state.last_ids.clone_from(&c.ids);
    Ok(())
}

fn sent_round(
    request: &CompiledRequest,
    new: &[crate::output::NewEvent],
    used: &(Vec<StepRecord>, Vec<ChatRecord>, Vec<ChatRecord>),
    state: &mut CaseState,
    st: &mut Occurrences,
) -> Result<(), TestCaseError> {
    sup::push_new(&mut state.events, new);
    let mut problems = mask_2_problems(request, &used.1, &used.2, &used.0);
    problems.extend(order_problems(request, &used.0));
    prop_assert!(problems.is_empty(), "{problems:?}");
    if used
        .0
        .iter()
        .any(|s| s.content.chars().count() <= K114_3_SHORT_RESULT_CHARS)
    {
        st.short_kept = st.short_kept.saturating_add(1);
    }
    if request
        .blocks
        .iter()
        .any(|b| b.kind == BlockKind::ToolResultMasked)
    {
        st.masks_seen = st.masks_seen.saturating_add(1);
    }
    let fresh = state
        .events
        .iter()
        .rev()
        .find_map(|e| anybot_domain::parse_collapsed(&e.kind).unwrap());
    if let Some(c) = fresh.filter(|c| c.reason == anybot_domain::ContextCollapseReason::Accumulated)
    {
        margin_round(&c, &used.0, state, st)?;
    }
    Ok(())
}

pub(crate) fn one_case(
    lens: &[usize],
    users: Vec<ChatRecord>,
    bots: Vec<ChatRecord>,
    usable: u64,
    st: &mut Occurrences,
) -> Result<(), TestCaseError> {
    let steps: Vec<StepRecord> = lens
        .iter()
        .enumerate()
        .map(|(i, len)| mk_step(i, *len))
        .collect();
    let steps = unique_step_refs(steps);
    let (users, bots, steps) = interleave(users, bots, steps);
    st.cases = st.cases.saturating_add(1);
    let mut state = CaseState {
        events: Vec::new(),
        last_ids: Vec::new(),
        pending: 0,
        floor: milli_of(sup::plan(usable).usable(), K114_3_ACCUMULATE_MARGIN_MILLI),
    };
    let mut sent_here = false;
    for index in 0..steps.len() {
        let used = used_upto(&steps, &users, &bots, index);
        let input = mask_input(&used.1, &used.2, &used.0, usable, state.events.clone());
        if let Compiled::Sent {
            request,
            events: new,
            ..
        } = compile(&input).map_err(|e| TestCaseError::fail(format!("{e:?}")))?
        {
            sent_here = true;
            sent_round(&request, &new, &used, &mut state, st)?;
        }
    }
    if sent_here {
        st.sent_cases = st.sent_cases.saturating_add(1);
    }
    Ok(())
}
