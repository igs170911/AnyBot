use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_protocol::{ComputerPlatform, ComputerRef};

use crate::runner::BotProfile;

#[must_use]
pub fn computer_code(platform: &ComputerPlatform) -> MessageCode {
    match platform {
        ComputerPlatform::BotMachine => MessageCode::TurnFixedComputerBotMachine,
        ComputerPlatform::Mac => MessageCode::TurnFixedComputerMac,
        ComputerPlatform::Linux => MessageCode::TurnFixedComputerLinux,
        ComputerPlatform::Windows => MessageCode::TurnFixedComputerWindows,
        ComputerPlatform::Ssh => MessageCode::TurnFixedComputerSsh,
        ComputerPlatform::Unknown => MessageCode::TurnFixedComputerUnknown,
    }
}

#[must_use]
pub fn lang_of(lang: Option<Lang>) -> Lang {
    match lang {
        Some(value) => value,
        None => Lang::En,
    }
}

const SAFETY_CODES: [MessageCode; 3] = [
    MessageCode::TurnFixedNoRetryAfterReject,
    MessageCode::TurnFixedUntrustedNotConsent,
    MessageCode::TurnFixedDraftOnly,
];

pub fn system_prompt(
    bot: &BotProfile,
    computer: &ComputerRef,
    lang: Lang,
) -> Result<String, anybot_messages::MessagesError> {
    let mut catalog = Catalog::new();
    let mut parts: Vec<String> = Vec::new();
    let mut identity = Params::new();
    identity = identity.with_text("name", &bot.name);
    identity = identity.with_text("timezone", &bot.timezone);
    parts.push(catalog.format(lang, &MessageCode::TurnFixedIdentity, &identity)?);
    if !bot.instructions.is_empty() {
        let params = Params::new().with_text("instructions", &bot.instructions);
        parts.push(catalog.format(lang, &MessageCode::TurnFixedInstructions, &params)?);
    }
    let code = computer_code(&computer.platform);
    parts.push(catalog.format(lang, &code, &Params::new())?);
    for safety in SAFETY_CODES {
        parts.push(catalog.format(lang, &safety, &Params::new())?);
    }
    parts.push(catalog.format(lang, &MessageCode::TurnFixedTaskEvidence, &Params::new())?);
    Ok(parts.join("\n"))
}

pub fn not_wired_text(lang: Lang, tool: &str) -> Result<String, anybot_messages::MessagesError> {
    let mut catalog = Catalog::new();
    let params = Params::new().with_text("tool", tool);
    catalog.format(lang, &MessageCode::TurnToolNotWired, &params)
}
