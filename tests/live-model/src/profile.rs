use std::path::{Path, PathBuf};

use anybot_foundation::secret::Secret;

pub const PROFILE_REL: &str = "target/live-model/profile.toml";
pub const REPORT_REL: &str = "target/live-model/format-report.txt";

pub struct LiveProfile {
    pub base_url: String,
    pub model: String,
    pub ca_pem: String,
    pub key: Secret,
}

#[must_use]
pub fn profile_dir(repo: &Path) -> PathBuf {
    repo.join("target").join("live-model")
}

pub fn load_profile(repo: &Path) -> Result<LiveProfile, String> {
    let path = repo.join(PROFILE_REL);
    if !path.is_file() {
        return Err(format!(
            "missing {PROFILE_REL}: create it with fields base_url (server address, e.g. https://host/v1), \
             model (model name), ca_file (self-signed certificate path, relative to target/live-model/), \
             key_file (API key file path, relative to target/live-model/)"
        ));
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("read {PROFILE_REL} failed: {e}"))?;
    let table: toml::Table =
        toml::from_str(&raw).map_err(|e| format!("parse {PROFILE_REL} failed: {e}"))?;
    let base_url = field(&table, "base_url")?;
    let model = field(&table, "model")?;
    let ca_file = field(&table, "ca_file")?;
    let key_file = field(&table, "key_file")?;
    let dir = profile_dir(repo);
    let ca_path = dir.join(&ca_file);
    let key_path = dir.join(&key_file);
    let ca_pem = std::fs::read_to_string(&ca_path)
        .map_err(|e| format!("read ca_file {ca_file} failed: {e}"))?;
    let key_raw = std::fs::read_to_string(&key_path)
        .map_err(|e| format!("read key_file {key_file} failed: {e}"))?;
    let key = Secret::from_untrusted(key_raw.trim().to_owned());
    Ok(LiveProfile {
        base_url,
        model,
        ca_pem,
        key,
    })
}

fn field(table: &toml::Table, name: &str) -> Result<String, String> {
    let value = table
        .get(name)
        .ok_or(format!("profile.toml lacks field {name}"))?;
    let text = value
        .as_str()
        .ok_or(format!("profile.toml field {name} must be a string"))?;
    if text.is_empty() {
        return Err(format!("profile.toml field {name} must not be empty"));
    }
    Ok(text.to_owned())
}
