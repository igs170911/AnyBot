use std::collections::{BTreeMap, BTreeSet};

const CORE: [&str; 3] = ["foundation", "protocol", "domain"];

#[must_use]
pub fn allowed_internal_deps() -> BTreeMap<String, BTreeSet<String>> {
    let mut m = BTreeMap::new();
    let mut add = |pkg: &str, deps: &[&str]| {
        m.insert(
            format!("anybot-{pkg}"),
            deps.iter()
                .map(|d| format!("anybot-{d}"))
                .collect::<BTreeSet<_>>(),
        );
    };
    add("foundation", &[]);
    add("protocol", &["foundation"]);
    add("domain", &CORE);
    add("trust", &CORE);
    add("models", &CORE);
    add("context", &["foundation", "protocol", "domain", "models"]);
    add("tools", &["foundation", "protocol", "domain", "trust"]);
    add("computers", &["foundation", "protocol", "domain", "trust"]);
    add("mcp", &["foundation", "protocol", "domain", "trust"]);
    add("blob", &["foundation"]);
    add("bus", &["foundation"]);
    add("store-server", &CORE);
    add("store-client", &CORE);
    add("sync", &["foundation", "protocol", "store-client"]);
    add("channels", &CORE);
    add("runner", &runner_deps());
    add(
        "external-agents",
        &["foundation", "protocol", "trust", "mcp"],
    );
    add(
        "server",
        &[
            "foundation",
            "protocol",
            "domain",
            "store-server",
            "blob",
            "bus",
            "channels",
            "messages",
        ],
    );
    add(
        "migrate-old",
        &["foundation", "protocol", "domain", "store-client"],
    );
    add("messages", &[]);
    add_tools(&mut m);
    m
}

fn runner_deps() -> Vec<&'static str> {
    vec![
        "foundation",
        "protocol",
        "domain",
        "context",
        "messages",
        "trust",
        "models",
        "tools",
        "mcp",
        "computers",
        "external-agents",
        "store-client",
        "sync",
        "bus",
        "blob",
        "channels",
    ]
}

fn add_tools(m: &mut BTreeMap<String, BTreeSet<String>>) {
    m.insert("depcheck".to_owned(), BTreeSet::new());
    m.insert("spec-trace".to_owned(), BTreeSet::new());
    m.insert("api-compat".to_owned(), BTreeSet::new());
    m.insert("iron-laws".to_owned(), BTreeSet::new());
    add_test_tools(m);
    m.insert(
        "live-model".to_owned(),
        [
            "test-harness".to_owned(),
            "anybot-foundation".to_owned(),
            "anybot-models".to_owned(),
        ]
        .into_iter()
        .collect(),
    );
    m.insert(
        "os-certs".to_owned(),
        BTreeSet::from(["anybot-foundation".to_owned()]),
    );
    m.insert(
        "api-docs".to_owned(),
        ["anybot-server".to_owned(), "anybot-protocol".to_owned()]
            .into_iter()
            .collect(),
    );
    m.insert("locale-gen".to_owned(), BTreeSet::new());
    m.insert("anybot-versions".to_owned(), BTreeSet::new());
}

fn add_test_tools(m: &mut BTreeMap<String, BTreeSet<String>>) {
    m.insert(
        "test-harness".to_owned(),
        BTreeSet::from([
            "anybot-foundation".to_owned(),
            "anybot-store-server".to_owned(),
        ]),
    );
    m.insert(
        "testenv".to_owned(),
        [
            "test-harness".to_owned(),
            "anybot-foundation".to_owned(),
            "anybot-server".to_owned(),
        ]
        .into_iter()
        .collect(),
    );
    m.insert(
        "runner-test".to_owned(),
        [
            "test-harness".to_owned(),
            "anybot-foundation".to_owned(),
            "anybot-protocol".to_owned(),
            "anybot-trust".to_owned(),
            "anybot-domain".to_owned(),
            "anybot-messages".to_owned(),
            "anybot-models".to_owned(),
            "anybot-runner".to_owned(),
            "serde-json".to_owned(),
            "sha2".to_owned(),
            "tokio".to_owned(),
        ]
        .into_iter()
        .collect(),
    );
    m.insert(
        "anybot-server-test".to_owned(),
        ["anybot-app-test".to_owned(), "anybot-domain".to_owned()]
            .into_iter()
            .collect(),
    );
    m.insert("anybot-sync-test".to_owned(), BTreeSet::new());
    m.insert(
        "anybot-app-test".to_owned(),
        [
            "anybot-foundation".to_owned(),
            "anybot-messages".to_owned(),
            "anybot-server".to_owned(),
            "test-harness".to_owned(),
        ]
        .into_iter()
        .collect(),
    );
}

#[must_use]
pub fn disallowed_app_deps(app: &str) -> Vec<&'static str> {
    match app {
        "server" => vec![
            "runner",
            "models",
            "tools",
            "computers",
            "mcp",
            "external-agents",
        ],
        "runner-service" | "agent" | "cli" => vec!["server", "store-server"],
        "desktop" | "desktop-context" => vec![
            "server",
            "store-server",
            "runner",
            "models",
            "tools",
            "computers",
            "mcp",
            "external-agents",
        ],
        _ => vec![],
    }
}

#[must_use]
pub fn forbidden_external_for_domain() -> BTreeSet<String> {
    [
        "tokio",
        "futures",
        "futures-util",
        "async-std",
        "sqlx",
        "reqwest",
        "hyper",
        "rustls",
        "tempfile",
        "notify",
        "walkdir",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[must_use]
pub fn packages_allowed_exemptions() -> BTreeSet<String> {
    [
        "anybot-foundation",
        "anybot-computers",
        "anybot-models",
        "anybot-mcp",
        "anybot-sync",
        "anybot-channels",
        "anybot-blob",
        "anybot-store-client",
        "depcheck",
        "spec-trace",
        "api-compat",
        "iron-laws",
        "api-docs",
        "locale-gen",
        "anybot-versions",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depcheck_domain_forbidden_external_list_is_complete() {
        let f = forbidden_external_for_domain();
        for x in ["tokio", "sqlx", "reqwest", "walkdir"] {
            assert!(f.contains(x));
        }
        assert!(!f.contains("serde"));
    }

    #[test]
    fn test_depcheck_exemption_packages_match_clippy_toml_locations() {
        let p = packages_allowed_exemptions();
        assert!(p.contains("anybot-foundation"));
        assert!(p.contains("iron-laws"));
        assert!(p.contains("api-docs"));
        assert!(p.contains("anybot-versions"));
        assert!(!p.contains("anybot-domain"));
        assert!(!p.contains("anybot-bus"));
    }

    #[test]
    fn test_depcheck_api_docs_may_depend_on_server_and_protocol_only() {
        let allowed = allowed_internal_deps();
        let want = BTreeSet::from(["anybot-server".to_owned(), "anybot-protocol".to_owned()]);
        assert_eq!(allowed.get("api-docs"), Some(&want));
    }

    #[test]
    fn test_depcheck_messages_has_no_internal_deps() {
        assert_eq!(
            allowed_internal_deps().get("anybot-messages"),
            Some(&BTreeSet::new())
        );
    }

    #[test]
    fn test_depcheck_app_table_covers_every_app_dir() {
        assert_eq!(disallowed_app_deps("server").len(), 6);
        assert_eq!(disallowed_app_deps("agent"), vec!["server", "store-server"]);
        assert_eq!(disallowed_app_deps("desktop").len(), 8);
        assert!(disallowed_app_deps("unknown").is_empty());
    }
}
