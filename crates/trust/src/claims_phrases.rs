pub const DONE_PHRASES: [&str; 16] = [
    "\u{5df2}\u{5b8c}\u{6210}",
    "\u{505a}\u{597d}\u{4e86}",
    "\u{641e}\u{5b9a}",
    "\u{5beb}\u{597d}\u{4e86}",
    "\u{6539}\u{597d}\u{4e86}",
    "\u{8655}\u{7406}\u{597d}\u{4e86}",
    "\u{8655}\u{7406}\u{5b8c}\u{7562}",
    "\u{5df2}\u{89e3}\u{6c7a}",
    "\u{4fee}\u{597d}\u{4e86}",
    "\u{5efa}\u{597d}\u{4e86}",
    "\u{8a2d}\u{597d}\u{4e86}",
    "\u{66f4}\u{65b0}\u{597d}\u{4e86}",
    "\u{6574}\u{7406}\u{597d}\u{4e86}",
    "\u{6e96}\u{5099}\u{597d}\u{4e86}",
    "\u{5168}\u{90e8}\u{5b8c}\u{6210}",
    "\u{5b8c}\u{6210}\u{4e86}",
];

pub const SEND_PHRASES: [&str; 30] = [
    "\u{5df2}\u{50b3}",
    "\u{5df2}\u{9001}\u{51fa}",
    "\u{5df2}\u{5bc4}\u{51fa}",
    "\u{5df2}\u{4e0a}\u{50b3}",
    "\u{5df2}\u{767c}\u{4f48}",
    "\u{5df2}\u{767c}\u{5e03}",
    "\u{5df2}\u{9644}\u{4e0a}",
    "\u{653e}\u{9032}\u{804a}\u{5929}\u{4e86}",
    "\u{50b3}\u{7d66}\u{4f60}\u{4e86}",
    "\u{50b3}\u{7d66}\u{4f60}",
    "\u{9001}\u{7d66}\u{4f60}\u{4e86}",
    "\u{767c}\u{7d66}\u{4f60}\u{4e86}",
    "\u{5bc4}\u{7d66}\u{4f60}\u{4e86}",
    "\u{767c}\u{9001}\u{5b8c}\u{6210}",
    "\u{50b3}\u{9001}\u{5b8c}\u{6210}",
    "\u{5df2}\u{767c}\u{9001}",
    "\u{5df2}\u{50b3}\u{9001}",
    "\u{5df2}\u{8f49}\u{767c}",
    "\u{5df2}\u{5206}\u{4eab}",
    "\u{5df2}\u{63d0}\u{4ea4}",
    "\u{5df2}\u{63a8}\u{9001}",
    "\u{5df2}\u{56de}\u{8986}",
    "\u{5df2}\u{901a}\u{77e5}",
    "\u{50b3}\u{597d}\u{4e86}",
    "\u{5bc4}\u{597d}\u{4e86}",
    "\u{9001}\u{597d}\u{4e86}",
    "\u{767c}\u{597d}\u{4e86}",
    "\u{50b3}\u{904e}\u{53bb}\u{4e86}",
    "\u{5bc4}\u{904e}\u{53bb}\u{4e86}",
    "\u{9001}\u{904e}\u{53bb}\u{4e86}",
];

pub const EN_DONE_PHRASES: [&str; 9] = [
    "has been completed",
    "all done",
    "is complete",
    "is completed",
    "task completed",
    "finished",
    "completed successfully",
    "i've finished",
    "i have completed",
];

pub const EN_SEND_PHRASES: [&str; 9] = [
    "i've sent",
    "i have sent",
    "has been sent",
    "already sent",
    "sent to you",
    "i've uploaded",
    "has been uploaded",
    "i've attached",
    "attached the file",
];

const PENDING_EXCLUSIONS: [&str; 9] = [
    "\u{9084}\u{6c92}",
    "\u{5c1a}\u{672a}",
    "\u{672a}\u{5b8c}\u{6210}",
    "not yet",
    "haven't",
    "hasn't",
    "not done",
    "incomplete",
    "not finished",
];

const CONDITIONAL_EXCLUSIONS: [&str; 14] = [
    "\u{5982}\u{679c}",
    "\u{5047}\u{5982}",
    "\u{5b8c}\u{6210}\u{5f8c}",
    "\u{4e4b}\u{5f8c}\u{6703}",
    "\u{61c9}\u{8a72}",
    "\u{53ef}\u{80fd}",
    "\u{4e5f}\u{8a31}",
    "if ",
    "once ",
    "will send",
    "will complete",
    "probably",
    "maybe",
    "might ",
];

const FAILURE_EXCLUSIONS: [&str; 14] = [
    "\u{5931}\u{6557}",
    "\u{51fa}\u{932f}",
    "\u{932f}\u{8aa4}",
    "\u{6c92}\u{6210}\u{529f}",
    "\u{7121}\u{6cd5}",
    "\u{505a}\u{4e0d}\u{5230}",
    "\u{88ab}\u{64cb}",
    "\u{88ab}\u{62d2}",
    "failed",
    "blocked",
    "rejected",
    "couldn't",
    "unable",
    "cannot",
];

const USER_ACTOR_EXCLUSIONS: [&str; 8] = [
    "\u{4f60}\u{5df2}",
    "\u{4f60}\u{50b3}",
    "\u{4f60}\u{4e0a}\u{50b3}",
    "\u{4f60}\u{5bc4}",
    "you have",
    "you already",
    "you uploaded",
    "you sent",
];

const REMOTE_OR_PAST_MARKERS: [&str; 12] = [
    "\u{4fe1}\u{7bb1}",
    "email",
    "\u{96f2}\u{7aef}",
    "cloud",
    "slack",
    "discord",
    "telegram",
    "\u{624b}\u{6a5f}",
    "\u{4e4b}\u{524d}",
    "\u{525b}\u{525b}",
    "\u{4e0a}\u{9762}",
    "previously",
];

const CHAT_FILE_WORDS: [&str; 12] = [
    "\u{6a94}\u{6848}",
    "\u{5831}\u{544a}",
    "\u{6587}\u{4ef6}",
    "\u{9644}\u{4ef6}",
    "\u{5716}\u{7247}",
    "\u{7167}\u{7247}",
    "\u{622a}\u{5716}",
    "file",
    "report",
    "document",
    "attachment",
    "image",
];

#[must_use]
pub(crate) fn is_non_claim_exclusion(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    let trimmed = lower.trim();
    if trimmed.is_empty()
        || trimmed.ends_with('?')
        || trimmed.ends_with('\u{ff1f}')
        || trimmed.contains("\u{55ce}\u{ff1f}")
        || trimmed.contains("\u{55ce}?")
        || trimmed.contains("\u{4e86}\u{55ce}")
    {
        return true;
    }
    if trimmed.contains("\u{7b49}") && trimmed.contains("\u{4e4b}\u{5f8c}") {
        return true;
    }
    has_any(&lower, &PENDING_EXCLUSIONS)
        || has_any(&lower, &CONDITIONAL_EXCLUSIONS)
        || has_any(&lower, &FAILURE_EXCLUSIONS)
        || has_any(&lower, &USER_ACTOR_EXCLUSIONS)
}

#[must_use]
pub(crate) fn has_done_claim(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    has_any(sentence, &DONE_PHRASES) || has_any(&lower, &EN_DONE_PHRASES)
}

#[must_use]
pub(crate) fn has_send_claim(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    has_any(sentence, &SEND_PHRASES) || has_any(&lower, &EN_SEND_PHRASES)
}

#[must_use]
pub(crate) fn has_file_in_chat_claim(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    if has_any(&lower, &REMOTE_OR_PAST_MARKERS) {
        return false;
    }
    if lower.contains("\u{653e}\u{9032}\u{804a}\u{5929}")
        || lower.contains("\u{5df2}\u{9644}\u{4e0a}")
        || lower.contains("i've attached")
        || lower.contains("attached the file")
    {
        return true;
    }
    let mentions_file = has_any(&lower, &CHAT_FILE_WORDS) || has_file_ext(&lower);
    let mentions_transfer = has_any(&lower, &SEND_PHRASES) || has_any(&lower, &EN_SEND_PHRASES);
    mentions_file && mentions_transfer
}

fn has_file_ext(lower: &str) -> bool {
    const EXTS: [&str; 8] = [
        ".pdf", ".md", ".txt", ".csv", ".json", ".png", ".jpg", ".zip",
    ];
    EXTS.iter().any(|ext| lower.contains(ext))
}

fn has_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pat| text.contains(pat))
}
