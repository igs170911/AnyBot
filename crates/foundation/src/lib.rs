use rustls_pki_types as _;

#[cfg(test)]
#[path = "certs_fault.rs"]
mod certs_fault;
#[cfg(test)]
#[path = "certs_store.rs"]
mod certs_store;

pub mod bootstrap;
pub mod bootstrap_file;
pub mod certs;
mod certs_check;
mod certs_os;
pub mod clock;
pub mod digest;
pub mod error;
pub mod ids;
pub mod master_key;
pub mod release;
pub mod secret;
pub mod secret_bytes;
pub mod startup;
#[cfg(test)]
mod startup_tests;

pub use bootstrap::ServerBootstrap;
pub use certs::{CertFault, TrustStore, TrustedCerts, classify_tls, classify_tls_error};
pub use clock::{Clock, ClockExpired, PinBoxFuture, SystemClock, TimestampMs, timeout};
pub use digest::ContentDigest;
pub use error::{
    CODE_BOOTSTRAP_CREDENTIALS_MISSING, CODE_BOOTSTRAP_DATABASE_MISSING,
    CODE_BOOTSTRAP_HOST_MISSING, CODE_BOOTSTRAP_LINE_MALFORMED,
    CODE_BOOTSTRAP_MASTER_KEY_MALFORMED, CODE_BOOTSTRAP_MISSING, CODE_BOOTSTRAP_NAME_DUPLICATE,
    CODE_BOOTSTRAP_NAME_MISSING, CODE_BOOTSTRAP_NAME_UNKNOWN, CODE_BOOTSTRAP_NOT_FOUND,
    CODE_BOOTSTRAP_PERMISSIONS_READ_FAILED, CODE_BOOTSTRAP_PERMISSIONS_TOO_OPEN,
    CODE_BOOTSTRAP_READ_FAILED, CODE_BOOTSTRAP_URL_MALFORMED, CODE_BOOTSTRAP_URL_SCHEME,
    CODE_CLIENT_DATA_DIR_MISSING, CODE_SERVER_STARTUP_LISTENING, FOUNDATION_MESSAGE_CODES,
    FoundationError, MessageParams, MessageRef,
};
pub use ids::{
    AccountId, BlobId, BotId, CallId, CardId, DeviceId, EventId, EvidenceId, RecordId, TaskId,
    TurnId,
};
pub use master_key::MasterKey;
pub use release::{
    DEV_APP_IDENTIFIER, DEV_APP_NAME, DEV_CHANNEL, STABLE_APP_IDENTIFIER, STABLE_APP_NAME,
    STABLE_CHANNEL, app_identifier, app_name, log_version, package_version, release_channel,
    release_is_dev, version_string,
};
pub use secret::Secret;
pub use secret_bytes::SecretBytes;
pub use startup::{
    ClientLaunch, ClientLaunchArgs, ServerCommand, ServerLaunchArgs, default_data_dir,
};
