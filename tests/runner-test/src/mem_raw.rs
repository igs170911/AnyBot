use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use anybot_foundation::ContentDigest;
use anybot_runner::{RawBlob, RawStore, RawStoreError, clamp_raw_bytes};

#[derive(Default)]
pub struct MemoryRaw {
    blobs: Mutex<Vec<(ContentDigest, RawBlob)>>,
}

impl MemoryRaw {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<(String, Vec<u8>)> {
        self.items()
            .into_iter()
            .map(|(d, v)| (d.hex().to_owned(), v))
            .collect()
    }

    #[must_use]
    pub fn items(&self) -> Vec<(ContentDigest, Vec<u8>)> {
        let Ok(blobs) = self.blobs.lock() else {
            return Vec::new();
        };
        blobs
            .iter()
            .map(|(d, blob)| (d.clone(), blob.bytes.clone()))
            .collect()
    }
}

impl RawStore for MemoryRaw {
    fn put(
        &self,
        bytes: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<ContentDigest, RawStoreError>> + Send + '_>> {
        let (kept, original_len) = clamp_raw_bytes(bytes);
        let digest = ContentDigest::sha256(&kept);
        if let Ok(mut blobs) = self.blobs.lock()
            && !blobs.iter().any(|(d, _)| d == &digest)
        {
            blobs.push((
                digest.clone(),
                RawBlob {
                    bytes: kept,
                    original_len,
                },
            ));
        }
        Box::pin(ready(Ok(digest)))
    }

    fn get(
        &self,
        digest: ContentDigest,
    ) -> Pin<Box<dyn Future<Output = Result<Option<RawBlob>, RawStoreError>> + Send + '_>> {
        let Ok(blobs) = self.blobs.lock() else {
            return Box::pin(ready(Ok(None)));
        };
        for (stored, blob) in &*blobs {
            if stored == &digest {
                return Box::pin(ready(Ok(Some(blob.clone()))));
            }
        }
        Box::pin(ready(Ok(None)))
    }
}

#[must_use]
pub fn share_raw(raw: &Arc<MemoryRaw>) -> Arc<dyn RawStore> {
    let shared: Arc<MemoryRaw> = Arc::clone(raw);
    shared
}
