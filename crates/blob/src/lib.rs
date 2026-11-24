use anybot_foundation::now_epoch_seconds;

pub fn write_local(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

#[derive(Debug, Clone)]
pub struct BlobRef {
    pub path: String,
    pub at_epoch_seconds: i64,
}

impl BlobRef {
    #[must_use]
    pub fn new(path: &str) -> Self {
        BlobRef {
            path: path.to_owned(),
            at_epoch_seconds: now_epoch_seconds(),
        }
    }
}
