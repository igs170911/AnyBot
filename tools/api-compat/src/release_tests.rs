use crate::dev_helpers::*;
use crate::sha::baseline;
use crate::sha::is_hex64;

#[test]
fn test_api_compat_selftest_blocks_once_a_released_version_exists() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-release")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    touch_changelog(&repo)?;
    git(&repo, &["tag", "v0.9.0-rc1"])?;
    let found = run(&repo, &seed_sha);
    assert!(found.is_empty(), "{found:?}");
    git(&repo, &["tag", "v1.0.0"])?;
    let found = run(&repo, &seed_sha);
    assert!(
        found
            .iter()
            .any(|line| line.contains("not implemented yet")),
        "{found:?}"
    );
    Ok(())
}

#[test]
fn test_api_compat_selftest_passes_recorded_description_only_changes() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-description")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    let base = fixture("asyncapi-base.yaml")?;
    let edited = base.replace(
        "the selftest protocol baseline",
        "the selftest protocol base",
    );
    write_api(&repo, (fixture("openapi.yaml"))?.as_str(), &edited)?;
    let sha = baseline(fixture("openapi.yaml")?.as_bytes(), edited.as_bytes());
    append_record(&repo, &sha, "a description was reworded")?;
    commit(&repo, "record the description change")?;
    let (found, notes) = run_notes(&repo, &seed_sha);
    assert!(is_hex64(&sha));
    assert!(found.is_empty(), "{found:?}");
    assert!(notes.is_empty(), "{notes:?}");
    Ok(())
}

#[test]
fn test_api_compat_selftest_needs_committed_baselines_at_the_ref() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-noref")?;
    git(&repo, &["init"])?;
    let found = run(&repo, "HEAD");
    assert!(!found.is_empty(), "{found:?}");
    Ok(())
}

#[test]
fn test_api_compat_selftest_blocks_a_ref_that_git_cannot_read() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-badref")?;
    seed(&repo)?;
    let _seed_sha = commit_sha(&repo)?;
    let found = run(&repo, "no-such-ref");
    assert!(!found.is_empty(), "{found:?}");
    Ok(())
}
#[test]
fn test_api_compat_selftest_blocks_an_empty_record() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-empty")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    std::fs::write(repo.join("api/breaking-changes.log"), "").map_err(|e| e.to_string())?;
    commit(&repo, "erase the record")?;
    let found = run(&repo, &seed_sha);
    assert!(found.iter().any(|line| line.contains("empty")), "{found:?}");
    Ok(())
}

#[test]
fn test_api_compat_selftest_blocks_malformed_changelog_lines() -> Result<(), String> {
    let repo = scratch_repo("api-compat-rust-malformed")?;
    seed(&repo)?;
    let seed_sha = commit_sha(&repo)?;
    std::fs::write(repo.join("api/breaking-changes.log"), "garbage line\n")
        .map_err(|e| e.to_string())?;
    commit(&repo, "write a garbage record")?;
    let found = run(&repo, &seed_sha);
    assert!(
        found.iter().any(|line| line.contains("not 64 hex digits")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|line| line.contains("YYYY-MM-DD")),
        "{found:?}"
    );
    Ok(())
}
