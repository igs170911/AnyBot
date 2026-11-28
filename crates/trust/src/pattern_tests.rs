use crate::pattern::{
    PatternError, PatternTarget, RulePattern, WebVia, allow_hits, ask_hits, matches_nothing,
    parse_pattern,
};

const HOME: &str = "/home/u";

fn pattern(text: &str) -> RulePattern {
    parse_pattern(text).unwrap()
}

fn command(line: &str) -> PatternTarget {
    PatternTarget::Command {
        line: line.to_owned(),
    }
}

fn path(text: &str) -> PatternTarget {
    PatternTarget::Path {
        path: text.to_owned(),
    }
}

fn web(url: &str, via: WebVia) -> PatternTarget {
    PatternTarget::Web {
        url: url.to_owned(),
        via,
    }
}

fn connector(server: &str, tool: &str) -> PatternTarget {
    PatternTarget::Connector {
        server: server.to_owned(),
        tool: tool.to_owned(),
    }
}

fn step(name: &str) -> PatternTarget {
    PatternTarget::Step {
        name: name.to_owned(),
    }
}

fn check_allow(text: &str, hits: &[PatternTarget], misses: &[PatternTarget]) {
    let rule = pattern(text);
    assert_eq!(hits.len(), 4, "{text}: need four hits");
    assert_eq!(misses.len(), 4, "{text}: need four misses");
    for target in hits {
        assert!(
            allow_hits(&rule, target, HOME),
            "{text} should allow {target:?}"
        );
    }
    for target in misses {
        assert!(
            !allow_hits(&rule, target, HOME),
            "{text} should not allow {target:?}"
        );
    }
}

#[test]
fn t_autonomy_patterns_1() {
    check_command_file_allows();
    check_connector_web_allows();
}

fn check_command_file_allows() {
    check_allow(
        "shell(git *)",
        &[
            command("git status"),
            command("git log --oneline"),
            command("git status && git diff"),
            command("git add . && git commit"),
        ],
        &[
            command("git status && rm x"),
            command("git log $(x)"),
            command("git `x`"),
            command("rm -rf x"),
        ],
    );
    check_allow(
        "file(~/Projects/**)",
        &[
            path("~/Projects/a.txt"),
            path("~/Projects/a/b/c.txt"),
            path("/home/u/Projects/x"),
            path("~/Projects/a/../b"),
        ],
        &[
            path("~/Projects/../x"),
            path("~/Other/a.txt"),
            path("/etc/passwd"),
            path("~/Projects"),
        ],
    );
    check_allow(
        "file(~/Projects/*)",
        &[
            path("~/Projects/a"),
            path("~/Projects/b.txt"),
            path("/home/u/Projects/c"),
            path("~/Projects/./d"),
        ],
        &[
            path("~/Projects/a/b"),
            path("~/Projects/a/b/c"),
            path("~/Projects/../Projects2/x"),
            path("/tmp/a"),
        ],
    );
}

fn check_connector_web_allows() {
    check_allow(
        "connector(gitlab__*)",
        &[
            connector("gitlab", "list_issues"),
            connector("gitlab", "create_issue"),
            connector("gitlab", "x"),
            connector("gitlab", "merge"),
        ],
        &[
            connector("github", "list_issues"),
            connector("gitlab2", "list_issues"),
            connector("slack", "gitlab__x"),
            connector("notgitlab", "x"),
        ],
    );
    check_allow(
        "web(domain:example.com)",
        &[
            web("https://example.com/", WebVia::Opened),
            web("https://sub.example.com/a?b=1", WebVia::Opened),
            web("http://EXAMPLE.com./x", WebVia::Opened),
            web("https://a.b.example.com", WebVia::Opened),
        ],
        &[
            web("https://example.com.evil.net/", WebVia::Opened),
            web("https://example.com/", WebVia::Redirect),
            web("https://example.com/", WebVia::LinkClick),
            web("https://example.com/", WebVia::Back),
        ],
    );
    check_allow(
        "tool(browser-click)",
        &[
            step("browser-click"),
            step("browser-click"),
            step("browser-click"),
            step("browser-click"),
        ],
        &[
            step("browser-type"),
            step("browser-click-all"),
            step("click"),
            step("Browser-click"),
        ],
    );
}

#[test]
fn ask_needs_only_one_segment() {
    let rule = pattern("shell(git *)");
    assert!(ask_hits(&rule, &command("git status && rm x"), HOME));
    assert!(ask_hits(&rule, &command("rm x && git status"), HOME));
    assert!(ask_hits(&rule, &command("git log $(x)"), HOME));
    assert!(!ask_hits(&rule, &command("rm x"), HOME));
}

#[test]
fn question_mark_matches_one_character() {
    let rule = pattern("shell(git ?)");
    assert!(allow_hits(&rule, &command("git a"), HOME));
    assert!(!allow_hits(&rule, &command("git ab"), HOME));
}

#[test]
fn bad_syntax_is_refused_and_unknown_targets_are_noticed() {
    assert_eq!(parse_pattern("shell(git *"), Err(PatternError::Unbalanced));
    assert_eq!(parse_pattern("shell git *)"), Err(PatternError::Unbalanced));
    assert_eq!(
        parse_pattern("mail(x)"),
        Err(PatternError::UnknownKind("mail".to_owned()))
    );
    assert_eq!(parse_pattern("shell()"), Err(PatternError::Empty));
    assert_eq!(parse_pattern("   "), Err(PatternError::Empty));
    assert!(matches_nothing(&pattern("web(example.com)")));
    assert!(!matches_nothing(&pattern("web(domain:example.com)")));
}
