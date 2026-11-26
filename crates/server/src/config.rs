use anybot_foundation::Secret;

#[derive(Debug)]
pub struct ServerConfig {
    pub listen_addresses: Vec<String>,
    pub listen_port: u16,
    pub public_url: Option<String>,
    pub invite_code: Option<Secret>,
    pub trust_proxy: Option<std::net::IpAddr>,
    pub proxy_secret: Option<Secret>,
    pub language: String,
}

pub const KEY_LISTEN_ADDRESSES: &str = "listen_addresses";
pub const KEY_LISTEN_PORT: &str = "listen_port";
pub const KEY_PUBLIC_URL: &str = "public_url";
pub const KEY_INVITE_CODE: &str = "invite_code";
pub const KEY_TRUST_PROXY: &str = "trust_proxy";
pub const KEY_PROXY_SECRET: &str = "proxy_secret";
pub const KEY_LANGUAGE: &str = "language";
pub const KEY_SERVER_VERSION: &str = "server_version";

pub const DEFAULT_LISTEN_ADDRESSES: &str = "127.0.0.1,::1";

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            listen_addresses: vec!["127.0.0.1".to_owned(), "::1".to_owned()],
            listen_port: crate::startup::DEFAULT_PORT,
            public_url: None,
            invite_code: None,
            trust_proxy: None,
            proxy_secret: None,
            language: "en".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigView {
    pub listen_addresses: Vec<String>,
    pub listen_port: u16,
    pub public_url: Option<String>,
    pub trust_proxy: Option<std::net::IpAddr>,
    pub proxy_secret_configured: bool,
    pub language: String,
}

impl ServerConfig {
    #[must_use]
    pub fn view(&self) -> ConfigView {
        ConfigView {
            listen_addresses: self.listen_addresses.clone(),
            listen_port: self.listen_port,
            public_url: self.public_url.clone(),
            trust_proxy: self.trust_proxy,
            proxy_secret_configured: self.proxy_secret.is_some(),
            language: self.language.clone(),
        }
    }

    pub fn set_public_url(&mut self, url: Option<String>) {
        self.public_url = url;
    }

    pub fn from_rows(rows: Vec<(String, String)>) -> Result<Self, String> {
        let mut config = ServerConfig::default();
        for (key, value) in rows {
            config.apply(&key, &value)?;
        }
        Ok(config)
    }

    #[must_use]
    pub fn rows(&self) -> Vec<(String, String)> {
        let mut rows = vec![
            (
                KEY_LISTEN_ADDRESSES.to_owned(),
                self.listen_addresses.join(","),
            ),
            (KEY_LISTEN_PORT.to_owned(), self.listen_port.to_string()),
            (KEY_LANGUAGE.to_owned(), self.language.clone()),
        ];
        push_row(&mut rows, KEY_PUBLIC_URL, self.public_url.as_deref());
        push_secret_row(&mut rows, KEY_INVITE_CODE, self.invite_code.as_ref());
        push_row(
            &mut rows,
            KEY_TRUST_PROXY,
            self.trust_proxy.map(|ip| ip.to_string()).as_deref(),
        );
        push_secret_row(&mut rows, KEY_PROXY_SECRET, self.proxy_secret.as_ref());
        rows
    }

    fn apply(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            KEY_LISTEN_ADDRESSES => self.listen_addresses = split_list(value),
            KEY_LISTEN_PORT => {
                let port = value
                    .parse::<u16>()
                    .map_err(|e| format!("listen_port: {e}"))?;
                self.listen_port = port;
            }
            KEY_PUBLIC_URL => self.public_url = non_empty(value),
            KEY_INVITE_CODE => self.invite_code = non_empty(value).map(Secret::from_untrusted),
            KEY_TRUST_PROXY => {
                let proxy = value
                    .parse::<std::net::IpAddr>()
                    .map_err(|e| format!("trust_proxy: {e}"))?;
                self.trust_proxy = Some(proxy);
            }
            KEY_PROXY_SECRET => self.proxy_secret = non_empty(value).map(Secret::from_untrusted),
            KEY_LANGUAGE => self.language = value.to_string(),
            _ => {}
        }
        Ok(())
    }
}

fn push_row(rows: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(text) = value {
        rows.push((key.to_string(), text.to_string()));
    }
}

fn push_secret_row(rows: &mut Vec<(String, String)>, key: &str, value: Option<&Secret>) {
    if let Some(secret) = value {
        rows.push((key.to_string(), secret.expose().to_string()));
    }
}

fn non_empty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}
