use crate::private_paths::is_private_path;

fn folder_hits() -> [&'static str; 26] {
    [
        "/home/alice/.local/share/AnyBot",
        "/home/alice/.local/share/AnyBot/keys.db",
        "/home/alice/.local/share/anybot/SUB/file.txt",
        "/home/alice/backup/vault.bin",
        "/data/vault.key",
        "/tmp/vault.db",
        "/tmp/vault.db-wal",
        "/tmp/vault.db-shm",
        "/repo/.git/HEAD",
        "/repo/.GIT/config",
        "/home/alice/.ssh/config",
        "/home/alice/.gnupg/secring.gpg",
        "/Users/alice/Library/Keychains/login.keychain-db",
        "/home/alice/.aws/credentials",
        "/home/alice/.config/gcloud/access_tokens.db",
        "/home/alice/.kube/config",
        "/home/alice/.docker/config.json",
        "/home/alice/.anybot/secret.json",
        "/home/alice/.local/share/keyrings/login.keyring",
        "/Users/alice/Library/Group Containers/2BUA8C4S2C.com.1password/data",
        "/Users/alice/Library/Application Support/1Password/data",
        "/home/alice/.config/1Password/settings",
        "/home/alice/.1password/config",
        "C:\\Users\\alice\\AppData\\Local\\1Password\\data",
        "C:\\Users\\alice\\AppData\\Roaming\\1Password\\data",
        "C:\\Users\\alice\\AppData\\Local\\Microsoft\\Credentials\\cred",
    ]
}

fn filename_and_legacy_hits() -> [&'static str; 25] {
    [
        "C:\\Users\\alice\\AppData\\Roaming\\Microsoft\\Credentials\\cred",
        "C:\\Users\\alice\\AppData\\Local\\Microsoft\\Vault\\item",
        "/workspace/.env",
        "/workspace/.ENV",
        "/workspace/.env.production",
        "/workspace/certs/server.pem",
        "/workspace/certs/private.KEY",
        "/home/alice/keys/id_rsa",
        "/home/alice/keys/id_rsa.pub",
        "/home/alice/keys/id_ed25519",
        "/home/alice/keys/id_ecdsa",
        "/home/alice/keys/id_dsa",
        "/home/alice/.netrc",
        "/home/alice/credentials",
        "/home/alice/credentials.json",
        "/home/alice/credentials.toml",
        "/home/alice/application_default_credentials.json",
        "/home/alice/.npmrc",
        "/home/alice/.pypirc",
        "/home/alice/.zsh_history",
        "/home/alice/.bash_history",
        "/etc/shadow",
        "/etc/sudoers",
        "/Users/alice/.securebookmarkswrite",
        "/home/alice/browser/cookies.sqlite",
    ]
}

fn non_hits() -> [&'static str; 20] {
    [
        "",
        "/workspace/src/environment.ts",
        "/workspace/notes/keys.md",
        "/workspace/notes/credentials_guide.md",
        "/workspace/docs/ssh-notes.md",
        "/workspace/src/docker_setup.rs",
        "/workspace/src/kube_notes.txt",
        "/workspace/src/git_helpers.rs",
        "/workspace/src/env_config.rs",
        "/workspace/src/pem_docs.txt",
        "/workspace/src/1password_guide.md",
        "/workspace/src/keychains_doc.md",
        "/workspace/src/gcloud_notes.md",
        "/workspace/src/vault_notes.md",
        "/workspace/src/netrc_notes.md",
        "/workspace/src/npmrc_docs.md",
        "/workspace/src/pypirc_docs.md",
        "/workspace/src/zsh_history_notes.md",
        "/workspace/src/bash_history_notes.md",
        "/home/alice/.local/share/OtherApp/data.txt",
    ]
}

#[test]
fn private_paths_name_patterns() {
    let home = "/home/alice";
    let anybot_dir = "/home/alice/.local/share/AnyBot";
    for path in folder_hits() {
        assert!(is_private_path(path, home, anybot_dir), "hit: {path}");
    }
    for path in filename_and_legacy_hits() {
        assert!(is_private_path(path, home, anybot_dir), "hit: {path}");
    }
    for path in non_hits() {
        assert!(!is_private_path(path, home, anybot_dir), "non-hit: {path}");
    }
}
