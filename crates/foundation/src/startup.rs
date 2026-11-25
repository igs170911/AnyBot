use std::path::Path;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::error::FoundationError;
use crate::secret::Secret;

#[derive(Parser)]
#[command(name = "anybot-client", version)]
pub struct ClientLaunchArgs {
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum ServerCommand {
    ResetPassword { account: String },
    Logs,
    UpdateBotComputers,
    MigrateOld,
}

#[derive(Parser)]
#[command(name = "anybot-server", version)]
pub struct ServerLaunchArgs {
    #[arg(long)]
    pub bootstrap: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<ServerCommand>,
}

#[derive(Debug)]
pub struct ClientLaunch {
    pub data_dir: PathBuf,
}

impl ClientLaunch {
    pub fn from_args(args: &ClientLaunchArgs) -> Result<Self, FoundationError> {
        match &args.data_dir {
            Some(dir) => Ok(ClientLaunch {
                data_dir: dir.clone(),
            }),
            None => default_data_dir()
                .map(|data_dir| ClientLaunch { data_dir })
                .ok_or(FoundationError::DataDirMissing(
                    "找不到本機資料夾的慣例位置：請用 --data-dir 指定".to_owned(),
                )),
        }
    }
}

#[must_use]
pub fn default_data_dir() -> Option<PathBuf> {
    let home = std::env::home_dir()?;
    let rel = match std::env::consts::OS {
        "macos" => "Library/Application Support/AnyBotNext",
        "linux" => ".local/share/anybotnext",
        "windows" => "AppData/Roaming/AnyBotNext",
        _ => return None,
    };
    let mut dir = home;
    for part in rel.split('/') {
        dir.push(part);
    }
    Some(dir)
}

#[derive(Debug)]
pub struct ServerBootstrap {
    pub user: String,
    pub password: Secret,
    pub host: String,
    pub port: u16,
    pub database: String,
}

impl ServerBootstrap {
    #[must_use]
    pub fn describe_db(&self) -> String {
        format!("{}/{}", self.host, self.database)
    }

    pub fn read(path: &Path) -> Result<Self, FoundationError> {
        let meta = std::fs::metadata(path).map_err(|e| {
            FoundationError::BootstrapNotFound(format!(
                "找不到啟動檔 {}（用 --bootstrap 指定）：{e}",
                path.display()
            ))
        })?;
        check_permissions(path, &meta)?;
        let text = std::fs::read_to_string(path).map_err(|e| {
            FoundationError::BootstrapNotFound(format!(
                "找不到啟動檔 {}（用 --bootstrap 指定）：{e}",
                path.display()
            ))
        })?;
        parse_connection(text.trim())
    }
}

fn check_permissions(path: &Path, meta: &std::fs::Metadata) -> Result<(), FoundationError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode = meta.mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(FoundationError::BootstrapPermissions(format!(
                "啟動檔的權限太開放（{mode:o}）：改成只有擁有者能讀（{}）",
                path.display()
            )));
        }
    }
    Ok(())
}

fn parse_connection(text: &str) -> Result<ServerBootstrap, FoundationError> {
    let rest = strip_scheme(text)?;
    let (userinfo, hostpart) = rest
        .split_once('@')
        .ok_or(FoundationError::BootstrapMalformed(
            "啟動檔缺少使用者與密碼".to_owned(),
        ))?;
    let (user, password) = split_userinfo(userinfo)?;
    let (hostpart, database) =
        hostpart
            .split_once('/')
            .ok_or(FoundationError::BootstrapMalformed(
                "啟動檔缺少資料庫名稱".to_owned(),
            ))?;
    if database.is_empty() {
        return Err(FoundationError::BootstrapMalformed(
            "啟動檔缺少資料庫名稱".to_owned(),
        ));
    }
    let (host, port) = split_hostport(hostpart)?;
    if host.is_empty() || user.is_empty() {
        return Err(FoundationError::BootstrapMalformed(
            "啟動檔缺少位址或使用者".to_owned(),
        ));
    }
    Ok(ServerBootstrap {
        user: user.to_owned(),
        password: Secret::from_untrusted(password.to_owned()),
        host: host.to_owned(),
        port,
        database: database.to_owned(),
    })
}

fn strip_scheme(text: &str) -> Result<&str, FoundationError> {
    text.strip_prefix("postgresql://")
        .ok_or(FoundationError::BootstrapMalformed(
            "啟動檔的連線位址不是 postgresql://".to_owned(),
        ))
}

fn split_userinfo(userinfo: &str) -> Result<(&str, &str), FoundationError> {
    userinfo
        .split_once(':')
        .ok_or(FoundationError::BootstrapMalformed(
            "啟動檔的密碼要放在連線位址裡".to_owned(),
        ))
}

fn split_hostport(hostport: &str) -> Result<(&str, u16), FoundationError> {
    match hostport.split_once(':') {
        Some((host, port)) => {
            let port = port
                .parse::<u16>()
                .map_err(|e| FoundationError::BootstrapMalformed(format!("埠格式錯：{e}")))?;
            Ok((host, port))
        }
        None => Ok((hostport, 5432)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_with(data_dir: Option<&str>) -> ClientLaunchArgs {
        let mut argv = vec!["anybot-client".to_owned()];
        if let Some(d) = data_dir {
            argv.push("--data-dir".to_owned());
            argv.push(d.to_owned());
        }
        ClientLaunchArgs::parse_from(argv)
    }

    #[test]
    fn test_startup_client_data_dir_flag_is_used_verbatim() {
        let launch = ClientLaunch::from_args(&args_with(Some("/tmp/x"))).unwrap();
        assert_eq!(launch.data_dir, PathBuf::from("/tmp/x"));
    }

    #[test]
    fn test_startup_client_without_flag_uses_convention_not_env_var() {
        let launch = ClientLaunch::from_args(&args_with(None)).unwrap();
        assert_eq!(launch.data_dir, default_data_dir().unwrap());
        assert!(!launch.data_dir.starts_with("/tmp/y"));
    }

    #[test]
    fn test_startup_client_rejects_data_dir_without_value() {
        let parsed = ClientLaunchArgs::try_parse_from(["anybot-client", "--data-dir"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_startup_data_dir_default_is_platform_convention() {
        let dir = default_data_dir().unwrap();
        let text = dir.to_string_lossy().to_lowercase();
        assert!(text.contains("anybotnext"));
        assert!(dir.starts_with(std::env::home_dir().unwrap()));
    }

    fn bootstrap_text() -> &'static str {
        "postgresql://anybot:pw-secret-1@localhost:5432/anybotnext\n"
    }

    #[test]
    fn test_startup_bootstrap_parses_all_parts_and_redacts_password() {
        let b = parse_connection(bootstrap_text().trim()).unwrap();
        assert_eq!(b.user, "anybot");
        assert_eq!(b.password.expose(), "pw-secret-1");
        assert_eq!(b.host, "localhost");
        assert_eq!(b.port, 5432);
        assert_eq!(b.database, "anybotnext");
        assert_eq!(b.describe_db(), "localhost/anybotnext");
        let shown = format!("{b:?}");
        assert!(shown.contains("Secret(遮罩)"));
        assert!(!shown.contains("pw-secret-1"));
    }

    #[test]
    fn test_startup_bootstrap_bad_urls_are_named_errors() {
        let cases = [
            "http://u:p@h/db",
            "postgresql://h/db",
            "postgresql://u:p@h/",
            "postgresql://u:p@h:99999/db",
            "postgresql://u@h:5432/db",
        ];
        for c in cases {
            assert!(parse_connection(c).is_err(), "{c}");
        }
    }

    #[test]
    fn test_startup_bootstrap_missing_file_says_bootstrap_flag() {
        let err = ServerBootstrap::read(Path::new("/nonexistent/boot.conf")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("找不到啟動檔"));
        assert!(msg.contains("--bootstrap"));
    }
}
