use anybot_protocol::ComputerCaps;

const PRIVATE_FOLDER_SEGMENTS: [&str; 8] = [
    ".git",
    ".ssh",
    ".gnupg",
    ".aws",
    ".kube",
    ".docker",
    ".anybot",
    ".1password",
];

const PRIVATE_FOLDER_SEQUENCES: [&[&str]; 11] = [
    &["library", "keychains"],
    &[".config", "gcloud"],
    &[".local", "share", "keyrings"],
    &["appdata", "local", "microsoft", "credentials"],
    &["appdata", "roaming", "microsoft", "credentials"],
    &["appdata", "local", "microsoft", "vault"],
    &["library", "group containers", "2bua8c4s2c.com.1password"],
    &["library", "application support", "1password"],
    &[".config", "1password"],
    &["appdata", "local", "1password"],
    &["appdata", "roaming", "1password"],
];

const PRIVATE_EXACT_NAMES: [&str; 14] = [
    ".env",
    ".netrc",
    "credentials",
    "credentials.json",
    "credentials.toml",
    "application_default_credentials.json",
    ".npmrc",
    ".pypirc",
    ".zsh_history",
    ".bash_history",
    "vault.bin",
    "vault.key",
    "vault.db",
    "vault.db-wal",
];

const PRIVATE_PREFIX_NAMES: [&str; 5] = [".env.", "id_rsa", "id_ed25519", "id_ecdsa", "id_dsa"];

const PRIVATE_SUFFIX_NAMES: [&str; 2] = [".pem", ".key"];

const LEGACY_PRIVATE_PATTERNS: [&str; 4] = [
    "/etc/shadow",
    "/etc/sudoers",
    "/.securebookmarkswrite",
    "cookies",
];

#[must_use]
pub fn is_private_path(real_path: &str, home: &str, anybot_dir: &str) -> bool {
    let norm = normalize_target(real_path, home);
    if norm.is_empty() {
        return false;
    }
    matches_anybot_dir(&norm, home, anybot_dir)
        || matches_folder_segments(&norm)
        || matches_filename(&norm)
        || matches_legacy(&norm)
}

#[must_use]
pub(crate) fn is_private_with_caps(
    real_path: &str,
    caps: &ComputerCaps,
    anybot_override: &str,
) -> bool {
    let anybot_dir = if anybot_override.is_empty() {
        &caps.anybot_dir
    } else {
        anybot_override
    };
    if is_private_path(real_path, &caps.home_dir, anybot_dir) {
        return true;
    }
    let norm = normalize_target(real_path, &caps.home_dir);
    if norm.is_empty() {
        return false;
    }
    let declared = caps.declared_private_paths();
    matches_declared_extra(&norm, &caps.home_dir, &declared)
}

fn matches_anybot_dir(norm: &str, home: &str, anybot_dir: &str) -> bool {
    if anybot_dir.trim().is_empty() {
        return false;
    }
    let ab = normalize_target(anybot_dir, home);
    if ab.is_empty() {
        return false;
    }
    norm == ab || norm.starts_with(&format!("{ab}/"))
}

fn matches_folder_segments(norm: &str) -> bool {
    let segs: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
    if segs.iter().any(|seg| PRIVATE_FOLDER_SEGMENTS.contains(seg)) {
        return true;
    }
    PRIVATE_FOLDER_SEQUENCES
        .iter()
        .any(|seq| has_segment_sequence(&segs, seq))
}

fn has_segment_sequence(segs: &[&str], seq: &[&str]) -> bool {
    if seq.is_empty() || segs.len() < seq.len() {
        return false;
    }
    segs.windows(seq.len()).any(|window| window == seq)
}

fn matches_filename(norm: &str) -> bool {
    let Some(name) = norm.split('/').rfind(|s| !s.is_empty()) else {
        return false;
    };
    if PRIVATE_EXACT_NAMES.contains(&name) || name == "vault.db-shm" {
        return true;
    }
    if PRIVATE_PREFIX_NAMES.iter().any(|p| name.starts_with(p)) {
        return true;
    }
    PRIVATE_SUFFIX_NAMES.iter().any(|s| name.ends_with(s))
}

fn matches_legacy(norm: &str) -> bool {
    LEGACY_PRIVATE_PATTERNS
        .iter()
        .any(|pattern| norm.contains(pattern))
}

fn matches_declared_extra(norm: &str, home: &str, declared: &[String]) -> bool {
    let segs: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
    for entry in declared {
        let extra = normalize_target(entry, home);
        if extra.is_empty() {
            continue;
        }
        if extra.starts_with('/') {
            if norm == extra || norm.starts_with(&format!("{extra}/")) {
                return true;
            }
            continue;
        }
        let extra_segs: Vec<&str> = extra.split('/').filter(|s| !s.is_empty()).collect();
        if has_segment_sequence(&segs, &extra_segs) {
            return true;
        }
    }
    false
}

fn normalize_target(path: &str, home: &str) -> String {
    let slash = path.trim().replace('\\', "/");
    let expanded = expand_tilde(&slash, home);
    let lower = expanded.to_lowercase();
    let collapsed = collapse_slashes(&lower);
    if collapsed.len() > 1 {
        collapsed.trim_end_matches('/').to_owned()
    } else {
        collapsed
    }
}

fn expand_tilde(path: &str, home: &str) -> String {
    let home_trimmed = home.trim().replace('\\', "/");
    let home_clean = home_trimmed.trim_end_matches('/');
    if home_clean.is_empty() {
        return path.to_owned();
    }
    if path == "~" {
        return home_clean.to_owned();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return format!("{home_clean}/{rest}");
    }
    path.to_owned()
}

fn collapse_slashes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_slash = false;
    for ch in text.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push('/');
            }
            prev_slash = true;
        } else {
            prev_slash = false;
            out.push(ch);
        }
    }
    out
}
