use anybot_models::{Role, ToolCall};

use crate::output::{Block, BlockKind, CODE_TURN_PROGRAM_NOTE, CompiledRequest};

const TOOL_PAIR_ERROR: &str = "context-tool-pair";

pub(super) fn join_system(request: &CompiledRequest) -> String {
    let mut parts = Vec::new();
    if !request.system.is_empty() {
        parts.push(request.system.clone());
    }
    for block in &request.blocks {
        if !is_prefix(block.kind) {
            continue;
        }
        if block.kind == BlockKind::System && block.text == request.system {
            continue;
        }
        parts.push(block.text.clone());
    }
    parts.join("\n")
}

pub(super) fn turn_messages(
    request: &CompiledRequest,
) -> Result<Vec<anybot_models::Message>, anybot_foundation::FoundationError> {
    let mut messages = Vec::new();
    let mut open: Vec<String> = Vec::new();
    for block in &request.blocks {
        if is_prefix(block.kind) || is_tail(block.kind) {
            continue;
        }
        messages.push(segment_message(block, &mut open)?);
    }
    if !open.is_empty() {
        return Err(pair_error());
    }
    close_turn(&mut messages, request);
    Ok(messages)
}

fn segment_message(
    block: &Block,
    open: &mut Vec<String>,
) -> Result<anybot_models::Message, anybot_foundation::FoundationError> {
    if block.kind == BlockKind::ToolIntent {
        return intent_message(block, open);
    }
    if is_result(block.kind) {
        return result_message(block, open);
    }
    Ok(plain_message(block.kind.role(), block.text.clone()))
}

fn intent_message(
    block: &Block,
    open: &mut Vec<String>,
) -> Result<anybot_models::Message, anybot_foundation::FoundationError> {
    let call = intent_call(block)?;
    open.push(call.id.clone());
    Ok(anybot_models::Message {
        role: Role::Assistant,
        text: block.text.clone(),
        tool_calls: vec![call],
        tool_call_id: None,
    })
}

fn result_message(
    block: &Block,
    open: &mut Vec<String>,
) -> Result<anybot_models::Message, anybot_foundation::FoundationError> {
    let id = result_id(block, open)?;
    Ok(anybot_models::Message {
        role: Role::Tool,
        text: block.text.clone(),
        tool_calls: Vec::new(),
        tool_call_id: Some(id),
    })
}

fn plain_message(role: Role, text: String) -> anybot_models::Message {
    anybot_models::Message {
        role,
        text,
        tool_calls: Vec::new(),
        tool_call_id: None,
    }
}

fn intent_call(block: &Block) -> Result<ToolCall, anybot_foundation::FoundationError> {
    let (Some(id), Some(name), Some(arguments)) = (
        block.call_id.clone(),
        block.tool_name.clone(),
        block.arguments.clone(),
    ) else {
        return Err(pair_error());
    };
    Ok(ToolCall {
        id,
        name,
        arguments: anybot_models::ToolArguments::Json(arguments),
    })
}

fn result_id(
    block: &Block,
    open: &mut Vec<String>,
) -> Result<String, anybot_foundation::FoundationError> {
    let Some(id) = block.call_id.clone() else {
        return Err(pair_error());
    };
    let Some(pos) = open.iter().position(|known| known == &id) else {
        return Err(pair_error());
    };
    open.remove(pos);
    Ok(id)
}

fn close_turn(messages: &mut Vec<anybot_models::Message>, request: &CompiledRequest) {
    if let Some(note) = program_note(request) {
        messages.push(note);
        return;
    }
    if ends_on_bot(messages) {
        messages.push(marker_only());
    }
}

fn ends_on_bot(messages: &[anybot_models::Message]) -> bool {
    messages
        .last()
        .is_none_or(|message| message.role == Role::Assistant)
}

fn marker_only() -> anybot_models::Message {
    plain_message(Role::User, CODE_TURN_PROGRAM_NOTE.to_owned())
}

fn program_note(request: &CompiledRequest) -> Option<anybot_models::Message> {
    let mut body = String::new();
    for block in &request.blocks {
        if !is_tail(block.kind) {
            continue;
        }
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(&block.text);
    }
    if body.is_empty() {
        return None;
    }
    Some(plain_message(
        Role::User,
        format!("{CODE_TURN_PROGRAM_NOTE}\n{body}"),
    ))
}

fn is_prefix(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::System
            | BlockKind::ToolDoc
            | BlockKind::Pinned
            | BlockKind::Constraint
            | BlockKind::Criterion
    )
}

fn is_tail(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::StateSummary
            | BlockKind::CurrentStep
            | BlockKind::OpenQuestion
            | BlockKind::PendingAction
            | BlockKind::Memory
    )
}

fn is_result(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::ToolResultFull | BlockKind::ToolResultMasked | BlockKind::ToolResultDuplicate
    )
}

fn pair_error() -> anybot_foundation::FoundationError {
    anybot_foundation::FoundationError::IdParse(TOOL_PAIR_ERROR.to_owned())
}

#[must_use]
pub(super) fn block_role(kind: BlockKind) -> Role {
    match kind {
        BlockKind::ToolResultFull
        | BlockKind::ToolResultMasked
        | BlockKind::ToolResultDuplicate => Role::Tool,
        BlockKind::ToolIntent | BlockKind::BotText => Role::Assistant,
        BlockKind::UserText
        | BlockKind::UserRef
        | BlockKind::PriorChatFull
        | BlockKind::PriorChatRef
        | BlockKind::System
        | BlockKind::ToolDoc
        | BlockKind::Pinned
        | BlockKind::Constraint
        | BlockKind::Criterion
        | BlockKind::CurrentStep
        | BlockKind::OpenQuestion
        | BlockKind::PendingAction
        | BlockKind::StateSummary
        | BlockKind::Memory => Role::User,
    }
}
