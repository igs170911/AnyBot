use crate::command_key::command_action_key;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionSpec {
    Command {
        command: String,
        computer: String,
    },
    WriteWorkspaceFile {
        computer: String,
    },
    ReadOutsideWorkspace {
        computer: String,
    },
    WatchScreen {
        computer: String,
    },
    OperateApp {
        computer: String,
        app: String,
    },
    RemoteComputer {
        computer: String,
        mode: String,
    },
    ConnectorTool {
        connector: String,
        tool: String,
        target: Option<String>,
    },
    SendMessage {
        recipients: Vec<String>,
    },
    ClickControl {
        computer: String,
        site: String,
        label: String,
    },
    BrowserSend {
        site: String,
        account: String,
    },
    InternalUrl {
        host: String,
        port: u16,
    },
    VaultLogin {
        site: String,
    },
    VaultContactFill {
        site: String,
    },
    VaultCardPay {
        card_last4: String,
        site: String,
    },
    PublishSite {
        site: String,
    },
    DeleteItem {
        kind: String,
        name: String,
    },
    AppleScriptOutsideBox {
        computer: String,
    },
    EditFile {
        computer: String,
        path: String,
    },
    ReadFile {
        computer: String,
        path: String,
    },
}

fn connector_key(connector: &str, tool: &str, target: Option<&String>) -> String {
    match target {
        Some(target) => format!("connector {connector} tool {tool} target {target}"),
        None => format!("connector {connector} tool {tool}"),
    }
}

#[must_use]
pub fn concrete_actions(spec: &ActionSpec) -> Vec<String> {
    match spec {
        ActionSpec::Command { command, computer } => {
            vec![format!(
                "command {} on {computer}",
                command_action_key(command)
            )]
        }
        ActionSpec::WriteWorkspaceFile { computer } => {
            vec![format!("write workspace file on {computer}")]
        }
        ActionSpec::ReadOutsideWorkspace { computer } => {
            vec![format!("read outside workspace on {computer}")]
        }
        ActionSpec::WatchScreen { computer } => {
            vec![format!("watch screen of {computer}")]
        }
        ActionSpec::OperateApp { computer, app } => {
            if app.is_empty() {
                vec![format!("operate whole screen of {computer}")]
            } else {
                vec![format!("operate app {app} on {computer}")]
            }
        }
        ActionSpec::RemoteComputer { computer, mode } => {
            vec![format!("{mode} of {computer}")]
        }
        ActionSpec::ConnectorTool {
            connector,
            tool,
            target,
        } => vec![connector_key(connector, tool, target.as_ref())],
        ActionSpec::SendMessage { .. }
        | ActionSpec::ClickControl { .. }
        | ActionSpec::BrowserSend { .. }
        | ActionSpec::InternalUrl { .. }
        | ActionSpec::VaultLogin { .. }
        | ActionSpec::VaultContactFill { .. }
        | ActionSpec::VaultCardPay { .. }
        | ActionSpec::PublishSite { .. }
        | ActionSpec::DeleteItem { .. }
        | ActionSpec::AppleScriptOutsideBox { .. } => tail_actions(spec),
        ActionSpec::EditFile { computer, path } => vec![format!("edit file {path} on {computer}")],
        ActionSpec::ReadFile { computer, path } => vec![format!("read file {path} on {computer}")],
    }
}

fn tail_actions(spec: &ActionSpec) -> Vec<String> {
    match spec {
        ActionSpec::Command { .. }
        | ActionSpec::WriteWorkspaceFile { .. }
        | ActionSpec::ReadOutsideWorkspace { .. }
        | ActionSpec::WatchScreen { .. }
        | ActionSpec::OperateApp { .. }
        | ActionSpec::RemoteComputer { .. }
        | ActionSpec::ConnectorTool { .. }
        | ActionSpec::EditFile { .. }
        | ActionSpec::ReadFile { .. } => vec![],
        ActionSpec::SendMessage { recipients } => {
            recipients.iter().map(|r| format!("send to {r}")).collect()
        }
        ActionSpec::ClickControl {
            computer,
            site,
            label,
        } => vec![format!("click {label} on {site} at {computer}")],
        ActionSpec::BrowserSend { site, account } => {
            vec![format!("browser send at {site} as {account}")]
        }
        ActionSpec::InternalUrl { host, port } => {
            vec![format!("internal url {host}:{port}")]
        }
        ActionSpec::VaultLogin { site } => vec![format!("vault login {site}")],
        ActionSpec::VaultContactFill { site } => {
            vec![format!("vault contact fill {site}")]
        }
        ActionSpec::VaultCardPay { card_last4, site } => {
            vec![format!("vault card {card_last4} pay {site}")]
        }
        ActionSpec::PublishSite { site } => vec![format!("publish site {site}")],
        ActionSpec::DeleteItem { kind, name } => {
            vec![format!("delete {kind} {name}")]
        }
        ActionSpec::AppleScriptOutsideBox { computer } => {
            vec![format!("apple script outside box on {computer}")]
        }
    }
}
