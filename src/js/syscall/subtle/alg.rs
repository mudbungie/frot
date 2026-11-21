//! The keyless and hash-keyed primitives, as pure functions of their bytes:
//! SHA-1/256/384/512, HMAC over each, and the two key-derivation functions
//! WebCrypto names, PBKDF2 and HKDF, over each (`bl-db8f`). All four are the
//! provider's. `None` means "not something this table computes"; the prelude
//! owns what a page is told about it. AES is [`super::aes`].

use std::num::NonZeroU32;

use aws_lc_rs::{digest, hkdf, hmac, pbkdf2};

/// One digest WebCrypto names, with its HMAC, PBKDF2 and HKDF twins.
struct Hash {
    digest: &'static digest::Algorithm,
    hmac: hmac::Algorithm,
    pbkdf2: pbkdf2::Algorithm,
    hkdf: hkdf::Algorithm,
}

fn hash(name: &str) -> Option<Hash> {
    let (digest, hmac, pbkdf2, hkdf) = match name {
        "SHA-1" => (
            &digest::SHA1_FOR_LEGACY_USE_ONLY,
            hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
            pbkdf2::PBKDF2_HMAC_SHA1,
            hkdf::HKDF_SHA1_FOR_LEGACY_USE_ONLY,
        ),
        "SHA-256" => (
            &digest::SHA256,
            hmac::HMAC_SHA256,
            pbkdf2::PBKDF2_HMAC_SHA256,
            hkdf::HKDF_SHA256,
        ),
        "SHA-384" => (
            &digest::SHA384,
            hmac::HMAC_SHA384,
            pbkdf2::PBKDF2_HMAC_SHA384,
            hkdf::HKDF_SHA384,
        ),
        "SHA-512" => (
            &digest::SHA512,
            hmac::HMAC_SHA512,
            pbkdf2::PBKDF2_HMAC_SHA512,
            hkdf::HKDF_SHA512,
        ),
        _ => return None,
    };
    Some(Hash {
        digest,
        hmac,
        pbkdf2,
        hkdf,
    })
}

pub fn digest(name: &str, data: &[u8]) -> Option<Vec<u8>> {
    hash(name).map(|h| digest::digest(h.digest, data).as_ref().to_vec())
}

pub fn hmac(name: &str, key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    hash(name).map(|h| {
        hmac::sign(&hmac::Key::new(h.hmac, key), data)
            .as_ref()
            .to_vec()
    })
}

/// The most PBKDF2 work one call may ask for, in HMAC iterations summed over
/// output blocks. The provider's loop cannot be interrupted, so the script CPU
/// budget (`EXEC_CPU_MS`) cannot stop it; this bounds it instead — a few
/// seconds at worst, and three times OWASP's 600 000-iteration SHA-256 advice
/// for a one-block key. Beyond it the derivation is refused, never truncated.
pub const PBKDF2_WORK: u64 = 2_000_000;

/// PBKDF2-HMAC: `len` bytes from a password. `None` for zero iterations or
/// more than [`PBKDF2_WORK`].
pub fn pbkdf2(name: &str, key: &[u8], salt: &[u8], rounds: u32, len: usize) -> Option<Vec<u8>> {
    let h = hash(name)?;
    let blocks = len.div_ceil(h.digest.output_len()) as u64;
    let rounds = NonZeroU32::new(rounds)
        .filter(|r| u64::from(r.get()).saturating_mul(blocks) <= PBKDF2_WORK)?;
    let mut out = vec![0; len];
    pbkdf2::derive(h.pbkdf2, rounds, salt, key, &mut out);
    Some(out)
}

/// An HKDF output length — the provider asks for it as a type.
struct Len(usize);

impl hkdf::KeyType for Len {
    fn len(&self) -> usize {
        self.0
    }
}

/// HKDF (RFC 5869), extract-then-expand: `len` bytes from input keying
/// material. `None` past the RFC's limit of 255 hash lengths.
pub fn hkdf(name: &str, key: &[u8], salt: &[u8], info: &[u8], len: usize) -> Option<Vec<u8>> {
    let prk = hkdf::Salt::new(hash(name)?.hkdf, salt).extract(key);
    let info = [info];
    let okm = prk.expand(&info, Len(len)).ok()?;
    let mut out = vec![0; len];
    okm.fill(&mut out).ok().map(|()| out)
}
