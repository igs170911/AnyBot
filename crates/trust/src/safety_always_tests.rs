use crate::safety_always::*;

#[test]
fn t_safety_always_3() {
    let command = ActionSpec::Command {
        command: "git push --force".to_owned(),
        computer: "work-mac".to_owned(),
    };
    assert_eq!(
        concrete_actions(&command),
        vec!["command git push on work-mac".to_owned()]
    );
    let npm = ActionSpec::Command {
        command: "npm install lodash".to_owned(),
        computer: "work-mac".to_owned(),
    };
    assert_eq!(
        concrete_actions(&npm),
        vec!["command npm install on work-mac".to_owned()]
    );
    let rm = ActionSpec::Command {
        command: "rm -rf build".to_owned(),
        computer: "work-mac".to_owned(),
    };
    assert_eq!(
        concrete_actions(&rm),
        vec!["command rm on work-mac".to_owned()]
    );
    let mail = ActionSpec::SendMessage {
        recipients: vec!["a@x.com".to_owned(), "b@x.com".to_owned()],
    };
    assert_eq!(
        concrete_actions(&mail),
        vec!["send to a@x.com".to_owned(), "send to b@x.com".to_owned()]
    );
}

fn row_examples() -> Vec<(ActionSpec, Vec<String>)> {
    vec![
        (
            ActionSpec::WriteWorkspaceFile {
                computer: "work-mac".to_owned(),
            },
            vec!["write workspace file on work-mac".to_owned()],
        ),
        (
            ActionSpec::ReadOutsideWorkspace {
                computer: "work-mac".to_owned(),
            },
            vec!["read outside workspace on work-mac".to_owned()],
        ),
        (
            ActionSpec::WatchScreen {
                computer: "work-mac".to_owned(),
            },
            vec!["watch screen of work-mac".to_owned()],
        ),
        (
            ActionSpec::OperateApp {
                computer: "work-mac".to_owned(),
                app: "Figma".to_owned(),
            },
            vec!["operate app Figma on work-mac".to_owned()],
        ),
        (
            ActionSpec::OperateApp {
                computer: "work-mac".to_owned(),
                app: String::new(),
            },
            vec!["operate whole screen of work-mac".to_owned()],
        ),
        (
            ActionSpec::RemoteComputer {
                computer: "home-linux".to_owned(),
                mode: "operate".to_owned(),
            },
            vec!["operate of home-linux".to_owned()],
        ),
        (
            ActionSpec::ConnectorTool {
                connector: "github".to_owned(),
                tool: "create_issue".to_owned(),
                target: None,
            },
            vec!["connector github tool create_issue".to_owned()],
        ),
        (
            ActionSpec::ConnectorTool {
                connector: "slack".to_owned(),
                tool: "send_message".to_owned(),
                target: Some("#team".to_owned()),
            },
            vec!["connector slack tool send_message target #team".to_owned()],
        ),
    ]
}

#[test]
fn test_concrete_action_table_rows_match_spec() {
    for (spec, expected) in row_examples() {
        assert_eq!(concrete_actions(&spec), expected);
    }
}

fn tail_examples_a() -> Vec<(ActionSpec, Vec<String>)> {
    vec![
        (
            ActionSpec::ClickControl {
                computer: "work-mac".to_owned(),
                site: "shop.example.com".to_owned(),
                label: "checkout".to_owned(),
            },
            vec!["click checkout on shop.example.com at work-mac".to_owned()],
        ),
        (
            ActionSpec::BrowserSend {
                site: "x.example.com".to_owned(),
                account: "@me".to_owned(),
            },
            vec!["browser send at x.example.com as @me".to_owned()],
        ),
    ]
}

fn tail_examples_b() -> Vec<(ActionSpec, Vec<String>)> {
    vec![
        (
            ActionSpec::InternalUrl {
                host: "192.168.1.10".to_owned(),
                port: 8080,
            },
            vec!["internal url 192.168.1.10:8080".to_owned()],
        ),
        (
            ActionSpec::VaultLogin {
                site: "example.com".to_owned(),
            },
            vec!["vault login example.com".to_owned()],
        ),
        (
            ActionSpec::VaultContactFill {
                site: "example.com".to_owned(),
            },
            vec!["vault contact fill example.com".to_owned()],
        ),
        (
            ActionSpec::VaultCardPay {
                card_last4: "1234".to_owned(),
                site: "shop.example.com".to_owned(),
            },
            vec!["vault card 1234 pay shop.example.com".to_owned()],
        ),
        (
            ActionSpec::PublishSite {
                site: "my-notes".to_owned(),
            },
            vec!["publish site my-notes".to_owned()],
        ),
        (
            ActionSpec::DeleteItem {
                kind: "routine".to_owned(),
                name: "daily-report".to_owned(),
            },
            vec!["delete routine daily-report".to_owned()],
        ),
        (
            ActionSpec::AppleScriptOutsideBox {
                computer: "work-mac".to_owned(),
            },
            vec!["apple script outside box on work-mac".to_owned()],
        ),
    ]
}

#[test]
fn test_concrete_action_tail_rows_match_spec() {
    for (spec, expected) in tail_examples_a() {
        assert_eq!(concrete_actions(&spec), expected);
    }
    for (spec, expected) in tail_examples_b() {
        assert_eq!(concrete_actions(&spec), expected);
    }
}
