extern crate clap as _;
extern crate serde_json as _;
extern crate sha2 as _;
extern crate yaml_serde as _;

use std::path::Path;

use api_compat::dev_helpers::*;
use api_compat::sha::baseline;

const EXE: &str = env!("CARGO_BIN_EXE_api-compat");

#[test]
fn test_compat_blocks_a_rewritten_hash_against_the_merge_base() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-merge-base-a1")?;
    let sha = seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    append_record(&repo, &seed_sha, "second baseline line")?;
    commit(&repo, "append the second line")?;
    start_branch(&repo)?;
    write_api(
        &repo,
        (fixture("openapi.yaml"))?.as_str(),
        (fixture("asyncapi-breaking.yaml"))?.as_str(),
    )?;
    let edited_hash = baseline(
        fixture("openapi.yaml")?.as_bytes(),
        fixture("asyncapi-breaking.yaml")?.as_bytes(),
    );
    let text = std::fs::read_to_string(repo.join("api/breaking-changes.log"))
        .map_err(|e| e.to_string())?;
    let rewritten = text.replace(&format!("sha256={sha}"), &format!("sha256={edited_hash}"));
    std::fs::write(repo.join("api/breaking-changes.log"), rewritten).map_err(|e| e.to_string())?;
    commit(&repo, "rewrite the second line hash without appending")?;
    let ref_text = merge_base(&repo)?;
    let found = run(&repo, &ref_text);
    assert!(
        found
            .iter()
            .any(|line| line.contains("api/breaking-changes.log")),
        "{found:?}"
    );
    let (text, ok) = binary(Path::new(EXE), &repo, &ref_text)?;
    assert!(!ok, "binary output: {text}");
    assert!(
        text.contains("api/breaking-changes.log"),
        "binary output: {text}"
    );
    Ok(())
}

#[test]
fn test_compat_accepts_an_appended_line_against_the_merge_base() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-merge-base-a2")?;
    let _sha = seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    append_record(&repo, &seed_sha, "second baseline line")?;
    commit(&repo, "append the second line")?;
    start_branch(&repo)?;
    write_api(
        &repo,
        (fixture("openapi.yaml"))?.as_str(),
        (fixture("asyncapi-breaking.yaml"))?.as_str(),
    )?;
    let edited_hash = baseline(
        fixture("openapi.yaml")?.as_bytes(),
        fixture("asyncapi-breaking.yaml")?.as_bytes(),
    );
    append_record(&repo, &edited_hash, "protocol edited and recorded")?;
    commit(&repo, "append the third line")?;
    let ref_text = merge_base(&repo)?;
    let (text, ok) = binary(Path::new(EXE), &repo, &ref_text)?;
    assert!(ok, "binary output: {text}");
    assert!(
        text.contains("breaking protocol change"),
        "binary output: {text}"
    );
    Ok(())
}

#[test]
fn test_compat_passes_when_nothing_changed() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-merge-base-a3")?;
    let _sha = seed(&repo)?;
    start_branch(&repo)?;
    commit(&repo, "an empty change on the branch")?;
    let ref_text = merge_base(&repo)?;
    let (text, ok) = binary(Path::new(EXE), &repo, &ref_text)?;
    assert!(ok, "binary output: {text}");
    Ok(())
}
