use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hkdf::Hkdf;
use p256::PublicKey;
use p256::ecdsa::SigningKey;
use sha2::Sha256;

pub const CODE_PUSH_PAYLOAD_SHORT: &str = "push-payload-short";
pub const CODE_PUSH_KEY_BASE64: &str = "push-key-base64";
pub const HEADER_LEN: usize = 16 + 4 + 1 + 65;

#[derive(Clone)]
pub struct SubscriptionKeys {
    pub private: SigningKey,
    public_uncompressed: Vec<u8>,
    pub auth: Vec<u8>,
}

impl SubscriptionKeys {
    #[must_use]
    pub fn from_private(private: &SigningKey, auth: Vec<u8>) -> Self {
        let public_uncompressed = private.verifying_key().to_sec1_bytes().to_vec();
        SubscriptionKeys {
            private: private.clone(),
            public_uncompressed,
            auth,
        }
    }

    pub fn generate() -> Result<Self, String> {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).map_err(|e| format!("signing key random failed: {e}"))?;
        let Ok(private) = SigningKey::from_bytes(
            seed.as_slice()
                .try_into()
                .map_err(|_e| "seed length wrong")?,
        ) else {
            return Err("signing key from bytes failed".to_owned());
        };
        let mut auth = [0u8; 16];
        getrandom::fill(&mut auth).map_err(|e| format!("auth secret random failed: {e}"))?;
        Ok(SubscriptionKeys::from_private(&private, auth.to_vec()))
    }

    #[must_use]
    pub fn public_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(&self.public_uncompressed)
    }

    #[must_use]
    pub fn auth_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(&self.auth)
    }
}

#[must_use]
pub fn subscription_endpoint(port: u16, subscription_id: &str) -> String {
    format!("http://127.0.0.1:{port}/push/{subscription_id}")
}

pub fn decrypt_aes128gcm(
    keys: &SubscriptionKeys,
    server_public_b64: &str,
    body: &[u8],
) -> Result<Vec<u8>, String> {
    if body.len() < HEADER_LEN {
        return Err(CODE_PUSH_PAYLOAD_SHORT.to_owned());
    }
    let Some(salt) = body.get(..16) else {
        return Err(CODE_PUSH_PAYLOAD_SHORT.to_owned());
    };
    let Some(idlen_byte) = body.get(20) else {
        return Err(CODE_PUSH_PAYLOAD_SHORT.to_owned());
    };
    let idlen = usize::from(*idlen_byte);
    if idlen != 65 {
        return Err(CODE_PUSH_PAYLOAD_SHORT.to_owned());
    }
    let as_public = decode_point(server_public_b64)?;
    let Some(ciphertext) = body.get(HEADER_LEN..) else {
        return Err(CODE_PUSH_PAYLOAD_SHORT.to_owned());
    };
    let shared = p256::elliptic_curve::ecdh::diffie_hellman(
        keys.private.as_nonzero_scalar(),
        as_public.as_affine(),
    );
    let as_affine: Vec<u8> = as_public.to_sec1_bytes().to_vec();
    let ikm = derive_ikm(
        shared.raw_secret_bytes().as_slice(),
        &keys.auth,
        &keys.public_uncompressed,
        as_affine
            .as_slice()
            .get(1..)
            .unwrap_or(as_affine.as_slice()),
    )?;
    let (cek, nonce) = derive_cek_nonce(ikm.as_ref(), salt)?;
    let cipher = <Aes128Gcm as KeyInit>::new_from_slice(cek.as_slice())
        .map_err(|_e| "cek length mismatch".to_owned())?;
    let record = cipher
        .decrypt(
            &Nonce::try_from(nonce.as_slice()).map_err(|_e| "nonce length mismatch".to_owned())?,
            Payload::from(ciphertext),
        )
        .map_err(|_e| "push record auth tag failed".to_owned())?;
    strip_pad(&record)
}

fn decode_point(encoded_b64: &str) -> Result<PublicKey, String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded_b64)
        .map_err(|e| format!("{CODE_PUSH_KEY_BASE64}: {e}"))?;
    PublicKey::from_sec1_bytes(&bytes).map_err(|e| format!("bad public key point: {e}"))
}

fn derive_ikm(
    shared: &[u8],
    auth: &[u8],
    ua_public: &[u8],
    as_public: &[u8],
) -> Result<Vec<u8>, String> {
    let prk_key = Hkdf::<Sha256>::new(Some(auth), shared);
    let mut info = Vec::with_capacity(
        ua_public
            .len()
            .saturating_add(as_public.len())
            .saturating_add(14),
    );
    info.extend_from_slice(b"WebPush: info\0");
    info.extend_from_slice(ua_public);
    info.extend_from_slice(as_public);
    let mut ikm = vec![0u8; 32];
    prk_key
        .expand(&info, &mut ikm)
        .map_err(|e| format!("ikm hkdf failed: {e}"))?;
    Ok(ikm)
}

fn derive_cek_nonce(ikm: &[u8], salt: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let prk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut cek = vec![0u8; 16];
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek)
        .map_err(|e| format!("cek hkdf failed: {e}"))?;
    let mut nonce = vec![0u8; 12];
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce)
        .map_err(|e| format!("nonce hkdf failed: {e}"))?;
    Ok((cek, nonce))
}

fn strip_pad(record: &[u8]) -> Result<Vec<u8>, String> {
    let mut body = record.to_vec();
    while body.last() == Some(&0) {
        body.pop();
    }
    if body.pop() != Some(2) {
        return Err("push record pad missing".to_owned());
    }
    Ok(body)
}

pub fn signing_key_from_pem(pem: &[u8]) -> Result<SigningKey, String> {
    let Ok(text) = std::str::from_utf8(pem) else {
        return Err("vapid pem is not utf8".to_owned());
    };
    <SigningKey as p256::elliptic_curve::pkcs8::DecodePrivateKey>::from_pkcs8_pem(text)
        .map_err(|e| format!("vapid pem key failed: {e}"))
}
