//! The primitives themselves, as pure functions of their bytes: SHA-1/256/384/
//! 512, HMAC over each, and AES-GCM with 128/192/256-bit keys. `None` means
//! "not something this table computes" (or, for [`gcm_open`], "did not
//! authenticate"); the prelude owns what a page is told about it.
//!
//! AES-GCM is the provider's safe AEAD, which fixes the nonce at 96 bits and the
//! tag at 128. Firefox also accepts other nonce and tag lengths (identity.md
//! §3.19); those are a declared residual (§11), refused in `subtle.js` before
//! they reach here.

use aws_lc_rs::{aead, digest, hmac};

/// The four digests WebCrypto names, each with its HMAC twin.
fn hash(name: &str) -> Option<(&'static digest::Algorithm, hmac::Algorithm)> {
    Some(match name {
        "SHA-1" => (
            &digest::SHA1_FOR_LEGACY_USE_ONLY,
            hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
        ),
        "SHA-256" => (&digest::SHA256, hmac::HMAC_SHA256),
        "SHA-384" => (&digest::SHA384, hmac::HMAC_SHA384),
        "SHA-512" => (&digest::SHA512, hmac::HMAC_SHA512),
        _ => return None,
    })
}

pub fn digest(name: &str, data: &[u8]) -> Option<Vec<u8>> {
    hash(name).map(|(alg, _)| digest::digest(alg, data).as_ref().to_vec())
}

pub fn hmac(name: &str, key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    hash(name).map(|(_, alg)| {
        hmac::sign(&hmac::Key::new(alg, key), data)
            .as_ref()
            .to_vec()
    })
}

/// The AEAD key and nonce for one call, or `None` for a key that is not 16, 24
/// or 32 bytes or a nonce that is not 12.
fn gcm(key: &[u8], iv: &[u8]) -> Option<(aead::LessSafeKey, aead::Nonce)> {
    let alg = match key.len() {
        16 => &aead::AES_128_GCM,
        24 => &aead::AES_192_GCM,
        32 => &aead::AES_256_GCM,
        _ => return None,
    };
    let key = aead::UnboundKey::new(alg, key).ok();
    let nonce = aead::Nonce::try_assume_unique_for_key(iv).ok();
    key.map(aead::LessSafeKey::new).zip(nonce)
}

/// Ciphertext with the 16-byte tag appended — WebCrypto's layout.
pub fn gcm_seal(key: &[u8], iv: &[u8], aad: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let (key, nonce) = gcm(key, iv)?;
    let mut buf = data.to_vec();
    let sealed = key.seal_in_place_append_tag(nonce, aead::Aad::from(aad), &mut buf);
    sealed.ok().map(|()| buf)
}

/// The plaintext of `data` (ciphertext + tag), or `None` if it does not
/// authenticate under this key, nonce and associated data.
pub fn gcm_open(key: &[u8], iv: &[u8], aad: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let (key, nonce) = gcm(key, iv)?;
    let mut buf = data.to_vec();
    let opened = key.open_in_place(nonce, aead::Aad::from(aad), &mut buf);
    let len = opened.ok()?.len();
    buf.truncate(len);
    Some(buf)
}
