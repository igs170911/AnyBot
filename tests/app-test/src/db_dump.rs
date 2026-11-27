use std::path::PathBuf;

use data_encoding::{BASE32, BASE32_NOPAD, BASE64, BASE64_NOPAD, HEXLOWER, HEXUPPER};
use test_harness::postgres::PG_VERSION_EXACT;
use test_harness::{Program, pg_install_cache_dir, run_program, test_dir};

use crate::{ServerDb, migrations_dir};

pub async fn migrator_up_to(test: &str, last: &str) -> Result<anybot_server::Migrator, String> {
    let dir = test_dir(test)?.join("migrations-up-to");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {} failed: {e}", dir.display()))?;
    let entries = std::fs::read_dir(migrations_dir()).map_err(|e| e.to_string())?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return Err(format!("unreadable migration name {}", path.display()));
        };
        if name <= last {
            std::fs::copy(&path, dir.join(name)).map_err(|e| e.to_string())?;
        }
    }
    anybot_server::Migrator::new(dir)
        .await
        .map_err(|e| e.to_string())
}

fn pg_dump_path() -> Result<PathBuf, String> {
    let version = PG_VERSION_EXACT.trim_start_matches('=');
    let path = pg_install_cache_dir()
        .join(version)
        .join("bin")
        .join(format!("pg_dump{}", std::env::consts::EXE_SUFFIX));
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("pg_dump not found at {}", path.display()))
    }
}

pub fn dump_database(db: &ServerDb) -> Result<String, String> {
    dump_with(db, Vec::new())
}

pub fn comparable_dump(db: &ServerDb) -> Result<String, String> {
    dump_with(db, vec![format!("--restrict-key={COMPARE_RESTRICT_KEY}")])
}

const COMPARE_RESTRICT_KEY: &str = "anybotcompare";

fn dump_with(db: &ServerDb, extra: Vec<String>) -> Result<String, String> {
    let bin = pg_dump_path()?;
    let mut args = vec![
        "--format=plain".to_owned(),
        "--no-owner".to_owned(),
        format!("--dbname={}", db.url),
    ];
    args.extend(extra);
    let outcome = run_program(&Program::new(&bin.display().to_string(), args), &db.dir)?;
    if outcome.code != 0 {
        return Err(format!("pg_dump exit {}: {}", outcome.code, outcome.stderr));
    }
    if !outcome.stdout.contains("server_config") {
        return Err("pg_dump output lacks the server_config table".to_owned());
    }
    Ok(outcome.stdout)
}

fn spellings(raw: &[u8]) -> Vec<(&'static str, String)> {
    let mut out = vec![
        ("hex", HEXLOWER.encode(raw)),
        ("HEX", HEXUPPER.encode(raw)),
        ("base64", BASE64.encode(raw)),
        ("base64-nopad", BASE64_NOPAD.encode(raw)),
        ("base32", BASE32.encode(raw)),
        ("base32-nopad", BASE32_NOPAD.encode(raw)),
    ];
    if let Ok(text) = std::str::from_utf8(raw) {
        out.push(("plain", text.to_owned()));
    }
    out
}

#[must_use]
pub fn found_spelling(haystack: &str, raw: &[u8]) -> Option<&'static str> {
    spellings(raw)
        .into_iter()
        .find(|(_label, spelled)| !spelled.is_empty() && haystack.contains(spelled.as_str()))
        .map(|(label, _spelled)| label)
}

#[must_use]
pub fn unkeyed_sha256(raw: &[u8]) -> Vec<u8> {
    ring::digest::digest(&ring::digest::SHA256, raw)
        .as_ref()
        .to_vec()
}

#[must_use]
pub fn found_raw_bytes(haystack: &str, raw: &[u8]) -> bool {
    !raw.is_empty()
        && haystack
            .as_bytes()
            .windows(raw.len())
            .any(|window| window == raw)
}

#[must_use]
pub fn found_digest(haystack: &str, raw: &[u8]) -> Option<&'static str> {
    let digest = unkeyed_sha256(raw);
    let spelled = [
        ("sha256-hex", HEXLOWER.encode(&digest)),
        ("sha256-HEX", HEXUPPER.encode(&digest)),
        ("sha256-base64", BASE64.encode(&digest)),
        ("sha256-base64-nopad", BASE64_NOPAD.encode(&digest)),
    ];
    spelled
        .into_iter()
        .find(|(_label, text)| haystack.contains(text.as_str()))
        .map(|(label, _text)| label)
}
