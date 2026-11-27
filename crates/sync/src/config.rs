use std::path::PathBuf;
use std::sync::Arc;

use anybot_foundation::{Clock, Secret, TrustedCerts};

use crate::reconnect_random::RandomSource;

pub struct SyncConfig {
    pub data_dir: PathBuf,
    pub server_url: String,
    pub credential: Secret,
    pub client_version: String,
    pub trusted: TrustedCerts,
    pub proxy: Option<String>,
    pub clock: Arc<dyn Clock>,
    pub random: Arc<dyn RandomSource>,
}
