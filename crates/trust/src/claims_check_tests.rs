use anybot_protocol::ActionOutcome;

use crate::claims_check::{StepFact, TurnFacts, check_reply};
use crate::claims_phrases::{DONE_PHRASES, EN_DONE_PHRASES, EN_SEND_PHRASES, SEND_PHRASES};
use crate::message_codes::{
    CODE_CLAIM_CHAT_COUNT, CODE_CLAIM_CHAT_DEAD_LINK, CODE_CLAIM_CHAT_NO_FILE,
    CODE_CLAIM_CHAT_STEP_FAILED,
};

fn failed_edit() -> TurnFacts {
    TurnFacts {
        steps: vec![StepFact {
            tool: "edit_file".to_owned(),
            read_only: false,
            outcome: ActionOutcome::Failed,
            exit_code: None,
            output_empty: false,
        }],
        files_placed: Vec::new(),
        sends: 0,
    }
}

fn failed_send() -> TurnFacts {
    TurnFacts {
        steps: vec![StepFact {
            tool: "send_file".to_owned(),
            read_only: false,
            outcome: ActionOutcome::Failed,
            exit_code: None,
            output_empty: false,
        }],
        files_placed: Vec::new(),
        sends: 0,
    }
}

#[test]
fn t_claims_check_4() {
    assert!(DONE_PHRASES.len() >= 15);
    assert!(SEND_PHRASES.len() >= 29);
    assert!(EN_DONE_PHRASES.len() + EN_SEND_PHRASES.len() >= 16);

    for phrase in DONE_PHRASES.iter().chain(EN_DONE_PHRASES.iter()) {
        let verdict = check_reply(phrase, &failed_edit());
        assert!(verdict.covers_claim, "done phrase missed: {phrase}");
        assert!(!verdict.verified, "done phrase not flagged: {phrase}");
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.code == CODE_CLAIM_CHAT_STEP_FAILED)
        );
    }

    for phrase in SEND_PHRASES.iter().chain(EN_SEND_PHRASES.iter()) {
        let verdict = check_reply(phrase, &failed_send());
        assert!(verdict.covers_claim, "send phrase missed: {phrase}");
        assert!(!verdict.verified, "send phrase not flagged: {phrase}");
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.code == CODE_CLAIM_CHAT_STEP_FAILED)
        );
    }

    let exclusions = [
        "\u{6211}\u{9084}\u{6c92}\u{5b8c}\u{6210}\u{3002}",
        "\u{5831}\u{544a}\u{5c1a}\u{672a}\u{5b8c}\u{6210}\u{3002}",
        "\u{5982}\u{679c}\u{5b8c}\u{6210}\u{4e86}\u{518d}\u{8aaa}\u{3002}",
        "\u{5047}\u{5982}\u{5df2}\u{50b3}\u{7d66}\u{4f60}\u{5c31}\u{597d}\u{4e86}\u{3002}",
        "\u{7b49}\u{8dd1}\u{5b8c}\u{4e4b}\u{5f8c}\u{5c31}\u{5b8c}\u{6210}\u{4e86}\u{3002}",
        "\u{5b8c}\u{6210}\u{5f8c}\u{6703}\u{50b3}\u{7d66}\u{4f60}\u{3002}",
        "\u{61c9}\u{8a72}\u{5df2}\u{5b8c}\u{6210}\u{4e86}\u{3002}",
        "\u{53ef}\u{80fd}\u{5df2}\u{7d93}\u{5bc4}\u{51fa}\u{4e86}\u{3002}",
        "\u{5df2}\u{5b8c}\u{6210}\u{4e86}\u{55ce}\u{ff1f}",
        "Has this been completed?",
        "\u{5df2}\u{5b8c}\u{6210}\u{7684}\u{90e8}\u{5206}\u{5148}\u{5beb}\u{597d}\u{ff0c}\u{4f46}\u{6e2c}\u{8a66}\u{9084}\u{6c92}\u{8dd1}\u{3002}",
        "All done with part 1, but part 2 is not done yet.",
        "\u{6539}\u{6a94}\u{5931}\u{6557}\u{ff0c}\u{6c92}\u{80fd}\u{5b8c}\u{6210}\u{3002}",
        "Tried to run it, but the command failed.",
        "`\u{5df2}\u{5b8c}\u{6210}` \u{548c} \"\u{5df2}\u{9001}\u{51fa}\" \u{53ea}\u{662f}\u{7bc4}\u{4f8b}\u{3002}",
        "\u{300c}\u{505a}\u{597d}\u{4e86}\u{300d}\u{5728}\u{6587}\u{4ef6}\u{88e1}\u{3002}",
        "\u{4f60}\u{5df2}\u{4e0a}\u{50b3}\u{6a94}\u{6848}\u{4e86}\u{3002}",
        "You already sent the report.",
    ];
    for sentence in exclusions {
        let verdict = check_reply(sentence, &failed_send());
        assert!(
            verdict.verified,
            "exclusion was wrongly flagged: {sentence} -> {:?}",
            verdict.reasons
        );
    }
}

#[test]
fn check_reply_covers_no_file_count_and_dead_link() {
    let empty = TurnFacts::default();
    let no_file = check_reply("\u{5831}\u{544a}\u{5df2}\u{50b3}\u{7d66}\u{4f60}", &empty);
    assert!(!no_file.verified);
    assert_eq!(
        no_file.reasons.first().map(|r| r.code.as_str()),
        Some(CODE_CLAIM_CHAT_NO_FILE)
    );

    let one_send = TurnFacts {
        steps: Vec::new(),
        files_placed: Vec::new(),
        sends: 1,
    };
    let count_mismatch = check_reply("\u{5df2}\u{9001}\u{51fa} 2 \u{5247}", &one_send);
    assert!(!count_mismatch.verified);
    assert_eq!(
        count_mismatch.reasons.first().map(|r| r.code.as_str()),
        Some(CODE_CLAIM_CHAT_COUNT)
    );

    let dead = check_reply("Here is [r](file:///Users/x/r.md)", &empty);
    assert!(!dead.verified);
    assert_eq!(
        dead.reasons.first().map(|r| r.code.as_str()),
        Some(CODE_CLAIM_CHAT_DEAD_LINK)
    );

    let matched = TurnFacts {
        steps: Vec::new(),
        files_placed: vec!["r.md".to_owned()],
        sends: 0,
    };
    let ok_link = check_reply("Here is [r](file:///Users/x/r.md)", &matched);
    assert!(ok_link.verified);
}
