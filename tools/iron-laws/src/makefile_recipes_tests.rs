use super::*;
use crate::makefile_scan::MAKEFILE_REL;
use crate::makefile_scan::split_top_level_semicolons;
use std::path::PathBuf;

const SHELL_LINE: &str = "SHELL := /bin/bash -o pipefail\n";

const INCIDENT: &str = "\tcargo nextest run --workspace --test-threads 1 2>&1 | tee target/nextest.log; ! grep -q \"ignoring unknown configuration keys\" target/nextest.log\n";

const BLOCK_CHAIN: &str = "\tif git cat-file -e HEAD:api/openapi.yaml 2>/dev/null; then git show HEAD:api/openapi.yaml > target/b.yaml; cargo run -p api-docs; oasdiff breaking target/b.yaml target/f.yaml -o ERR; fi\n";

const FLOCK_CHAIN: &str = "\texec 9>\"$$LOCK\"; flock -x 9; $$MAKE _test-body\n";

const MAKE_SHELL_PIPE: &str = "\tfor manifest in $(shell cargo metadata --format-version 1 --no-deps | jq -r '.packages[].manifest_path'); do (cd \"$$(dirname $$manifest)\" && cargo clippy) || exit 1; done\n";

const CLEAN_RECIPES: &str = "\tbash tools/test-gate.sh expect-fail target/g.log bash -c 'echo \"ignoring unknown configuration keys\" >&2; exit 0'\n\
\twhile read -r manifest; do (cd \"$$(dirname \"$$manifest\")\" && cargo clippy --all-targets -- -D warnings) || exit 1; done < target/clippy-manifests.txt\n\
\tif git cat-file -e HEAD:api/openapi.yaml 2>/dev/null; then git show HEAD:api/openapi.yaml > target/b.yaml && cargo run --locked -p api-docs -- openapi target/f.yaml && oasdiff breaking target/b.yaml target/f.yaml -o ERR; fi\n\
\tfor pair in \"no-5xx.yaml rule/operation-has-5xx\"; do file=$${pair%% *}; rule=$${pair##* }; out=$$(cargo lint $$file 2>&1); rc=$$?; if [ $$rc -eq 0 ]; then echo \"selftest: $$file should have been blocked\"; exit 1; fi; if ! echo \"$$out\" | grep -F \"$$rule\" > /dev/null; then echo \"selftest: $$file did not hit $$rule\"; exit 1; fi; done\n\
\t! test -s target/reject.txt || { echo \"disallowed license; see target/reject.txt\"; exit 1; }\n\
\tbash tools/test-gate.sh expect-pass target/g.log bash -c 'echo clean gate output'\n";

fn scratch_repo(tag: &str) -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/scratch")
        .join(tag);
    assert!(std::fs::create_dir_all(&repo).is_ok());
    repo
}

fn violations_for(tag: &str, body: &str) -> Vec<String> {
    let repo = scratch_repo(tag);
    assert!(std::fs::write(repo.join(MAKEFILE_REL), format!("{SHELL_LINE}{body}"),).is_ok());
    let out = violations(&repo);
    assert!(std::fs::remove_file(repo.join(MAKEFILE_REL)).is_ok());
    out
}

#[test]
fn test_make_recipes_gate_blocks_the_incident_line() {
    let v = violations_for("iron-make-incident", INCIDENT);
    assert_eq!(v.len(), 1);
    assert!(
        v.first()
            .is_some_and(|x| x.contains("`;` between") && x.contains("tee target/nextest.log")),
        "{v:?}"
    );
}

#[test]
fn test_make_recipes_gate_blocks_chains_inside_conditional_blocks() {
    let v = violations_for("iron-make-block", BLOCK_CHAIN);
    assert_eq!(v.len(), 2);
    assert!(
        v.iter().any(|x| x.contains("git show")) && v.iter().any(|x| x.contains("cargo run")),
        "{v:?}"
    );
}

#[test]
fn test_make_recipes_gate_blocks_sequenced_lock_commands() {
    let v = violations_for("iron-make-flock", FLOCK_CHAIN);
    assert_eq!(v.len(), 2);
    assert!(v.iter().any(|x| x.contains("flock -x 9")), "{v:?}");
}

#[test]
fn test_make_recipes_gate_blocks_pipes_inside_make_shell_calls() {
    let v = violations_for("iron-make-shellpipe", MAKE_SHELL_PIPE);
    assert_eq!(v.len(), 1);
    assert!(
        v.first().is_some_and(|x| x.contains("$(shell ...)")),
        "{v:?}"
    );
}

#[test]
fn test_make_recipes_gate_blocks_pipes_without_pipefail_shell() {
    let repo = scratch_repo("iron-make-recipes-nopipefail");
    assert!(
        std::fs::write(
            repo.join(MAKEFILE_REL),
            "SHELL := /bin/bash\nfmt:\n\tcargo fmt --all | tee target/fmt.log\n",
        )
        .is_ok()
    );
    let v = violations(&repo);
    assert_eq!(v.len(), 1);
    assert!(v.first().is_some_and(|x| x.contains("pipefail")), "{v:?}");
    assert!(std::fs::remove_file(repo.join(MAKEFILE_REL)).is_ok());
}

#[test]
fn test_make_recipes_gate_accepts_the_fixed_recipe_shapes() {
    assert_eq!(
        violations_for("iron-make-clean", CLEAN_RECIPES),
        Vec::<String>::new()
    );
}

#[test]
fn test_make_recipes_gate_ignores_semicolons_inside_quotes() {
    let segments = split_top_level_semicolons("bash -c 'exit 1; echo hi'");
    assert_eq!(segments.len(), 1);
    assert_eq!(classify(segments.first().unwrap()), Segment::Failable);
}

#[test]
fn test_make_recipes_gate_classifies_segments() {
    assert_eq!(classify("exit 3"), Segment::Terminal);
    assert_eq!(classify("fi"), Segment::Structural);
    assert_eq!(classify("< target/f.txt"), Segment::Structural);
    assert_eq!(classify("if [ $$rc -eq 0 ]"), Segment::Condition);
    assert_eq!(classify("then echo hi"), Segment::Failable);
    assert_eq!(classify("file=$${pair%% *}"), Segment::SafeAssignment);
    assert_eq!(classify("rc=$$?"), Segment::SafeAssignment);
    assert_eq!(classify("out=$$(cargo lint f)"), Segment::Failable);
    assert_eq!(classify("LC_ALL=C comm -23 a b"), Segment::Failable);
    assert_eq!(classify("! grep -q x log"), Segment::Failable);
    assert_eq!(classify("cargo build && cargo test"), Segment::Failable);
}

#[test]
fn test_make_recipes_gate_joins_continuation_lines() {
    let lines = logical_recipe_lines("t:\n\ta \\\n\tb; \\\n\tfi\n");
    assert_eq!(lines.len(), 1);
    assert!(
        lines
            .first()
            .is_some_and(|(_, body)| body.contains("a b") && body.ends_with("fi")),
        "{lines:?}"
    );
}

#[test]
fn test_make_recipes_real_repo_is_clean() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let v = violations(&repo);
    assert_eq!(v, Vec::<String>::new(), "{v:?}");
}
