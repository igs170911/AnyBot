#![cfg(test)]

use crate::dev_helpers::*;
use crate::sha::baseline;

#[test]
fn test_api_compat_required_list_growth_is_breaking() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-required")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    write_api(
        &repo,
        (fixture("openapi.yaml"))?.as_str(),
        (fixture("asyncapi-required.yaml"))?.as_str(),
    )?;
    let edited = baseline(
        fixture("openapi.yaml")?.as_bytes(),
        fixture("asyncapi-required.yaml")?.as_bytes(),
    );
    append_record(&repo, &edited, "a payload property became required")?;
    commit(&repo, "add a required list")?;
    let (found, notes) = run_notes(&repo, &seed_sha);
    assert!(found.is_empty(), "{found:?}");
    assert!(
        notes
            .iter()
            .any(|line| line.contains("a new required list")),
        "{notes:?}"
    );
    Ok(())
}

#[test]
fn test_api_compat_selftest_blocks_an_untracked_record_file() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-untracked")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    git(&repo, &["rm", "--cached", "api/breaking-changes.log"])?;
    git(
        &repo,
        &[
            "-c",
            "user.email=selftest@invalid",
            "-c",
            "user.name=selftest",
            "commit",
            "-m",
            "drop the record from the index",
        ],
    )?;
    let found = run(&repo, &seed_sha);
    assert!(
        found.iter().any(|line| line.contains("not tracked by git")),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn test_api_compat_selftest_blocks_breaking_changes_without_a_record() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-breaking")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    write_api(
        &repo,
        (fixture("openapi.yaml"))?.as_str(),
        (fixture("asyncapi-breaking.yaml"))?.as_str(),
    )?;
    commit(&repo, "remove a payload property")?;
    let found = run(&repo, &seed_sha);
    assert!(
        found.iter().any(|line| line.contains("must grow")),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn test_api_compat_selftest_passes_recorded_breaking_changes_with_a_report() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-recorded-breaking")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    write_api(
        &repo,
        (fixture("openapi.yaml"))?.as_str(),
        (fixture("asyncapi-breaking.yaml"))?.as_str(),
    )?;
    let sha = baseline(
        fixture("openapi.yaml")?.as_bytes(),
        fixture("asyncapi-breaking.yaml")?.as_bytes(),
    );
    append_record(&repo, &sha, "a payload property was removed")?;
    commit(&repo, "record the breaking change")?;
    let (found, notes) = run_notes(&repo, &seed_sha);
    assert!(found.is_empty(), "violations {found:?} notes {notes:?}");
    assert!(
        notes
            .iter()
            .any(|line| line.contains("breaking protocol change")),
        "violations {found:?} notes {notes:?}"
    );
    Ok(())
}

#[test]
fn test_api_compat_selftest_blocks_baseline_changes_without_a_record() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-norecord")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    write_api(
        &repo,
        (fixture("openapi.yaml"))?.as_str(),
        (fixture("asyncapi-added.yaml"))?.as_str(),
    )?;
    commit(&repo, "add a payload property")?;
    let found = run(&repo, &seed_sha);
    assert!(
        found.iter().any(|line| line.contains("must grow")),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn test_api_compat_selftest_blocks_openapi_changes_without_a_record() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-openapi")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    let mut openapi = fixture("openapi.yaml")?;
    openapi.push_str("# an extra comment line changes the baseline hash\n");
    write_api(&repo, &openapi, (fixture("asyncapi-base.yaml"))?.as_str())?;
    commit(&repo, "touch the openapi file")?;
    let found = run(&repo, &seed_sha);
    assert!(
        found.iter().any(|line| line.contains("must grow")),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn test_api_compat_selftest_blocks_edits_of_older_record_lines() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-prefix")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    write_api(
        &repo,
        (fixture("openapi.yaml"))?.as_str(),
        (fixture("asyncapi-added.yaml"))?.as_str(),
    )?;
    let sha = baseline(
        fixture("openapi.yaml")?.as_bytes(),
        fixture("asyncapi-added.yaml")?.as_bytes(),
    );
    let old = std::fs::read_to_string(repo.join("api/breaking-changes.log"))
        .map_err(|e| e.to_string())?;
    let edited = old.replace("fixtures seeded", "seed renamed");
    let text = format!("{edited}2026-01-02 | sha256={sha} | a property was added | appended\n");
    std::fs::write(repo.join("api/breaking-changes.log"), text).map_err(|e| e.to_string())?;
    commit(&repo, "rewrite an old record line")?;
    let found = run(&repo, &seed_sha);
    assert!(
        found.iter().any(|line| line.contains("prefix")),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn test_api_compat_selftest_blocks_a_tail_hash_that_does_not_match_the_baselines()
-> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-tailhash")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    append_record(&repo, &"0".repeat(64), "a property was added")?;
    commit(&repo, "record with the wrong hash")?;
    let found = run(&repo, &seed_sha);
    assert!(
        found
            .iter()
            .any(|line| line.contains("regenerated baseline must record")),
        "{found:?}"
    );
    Ok(())
}
