use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sad_web_push::VapidSignatureBuilder;

use anybot_foundation::Secret;

use super::PushError;

const SCALAR_LEN: usize = 32;
const GENERATE_ATTEMPTS: usize = 8;

pub struct VapidKey {
    scalar: Secret,
}

impl std::fmt::Debug for VapidKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VapidKey(..)")
    }
}

impl VapidKey {
    pub fn generate() -> Result<VapidKey, PushError> {
        for _attempt in 0..GENERATE_ATTEMPTS {
            let mut raw = [0u8; SCALAR_LEN];
            getrandom::fill(&mut raw).map_err(|source| PushError::Random(source.to_string()))?;
            let encoded = URL_SAFE_NO_PAD.encode(raw);
            if let Ok(key) = VapidKey::from_scalar(Secret::from_untrusted(encoded)) {
                return Ok(key);
            }
        }
        Err(PushError::VapidKey)
    }

    pub fn from_scalar(scalar: Secret) -> Result<VapidKey, PushError> {
        VapidSignatureBuilder::from_base64_no_sub(scalar.expose())
            .map_err(|_| PushError::VapidKey)?;
        Ok(VapidKey { scalar })
    }

    pub fn public_b64(&self) -> Result<String, PushError> {
        let builder = VapidSignatureBuilder::from_base64_no_sub(self.scalar.expose())
            .map_err(|_| PushError::VapidKey)?;
        Ok(URL_SAFE_NO_PAD.encode(builder.get_public_key()))
    }

    #[must_use]
    pub fn scalar(&self) -> &Secret {
        &self.scalar
    }
}
