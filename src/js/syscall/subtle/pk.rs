//! The elliptic-curve primitives under WebCrypto's public-key half (`bl-0cae`):
//! ECDSA and ECDH over P-256/384/521, Ed25519 and X25519, as pure functions of
//! their bytes, every one the provider's. A key crosses this boundary as its
//! raw material — the private scalar or seed, and the public point (the
//! uncompressed SEC 1 encoding for the NIST curves, 32 bytes for the 25519
//! pair) — never as a provider object, so nothing is held between calls. The
//! DER containers (`spki`, `pkcs8`) and every WebCrypto rule are the
//! prelude's. `None` (or `false`) means the operation cannot be performed on
//! these bytes.

use aws_lc_rs::agreement::{self, ParsedPublicKey, PrivateKey, UnparsedPublicKey};
use aws_lc_rs::digest::{self, Digest};
use aws_lc_rs::encoding::{AsBigEndian, Curve25519SeedBin, EcPrivateKeyBin};
use aws_lc_rs::signature::{self, EcdsaKeyPair, Ed25519KeyPair, KeyPair};

mod point;

/// The ECDSA pair whose digest matches a curve: how the provider signs it,
/// verifies it, and the digest length it signs.
type Ecdsa = (
    &'static signature::EcdsaSigningAlgorithm,
    &'static signature::EcdsaVerificationAlgorithm,
    &'static digest::Algorithm,
);

/// A curve by its WebCrypto name: its agreement algorithm, and for the NIST
/// curves the matching ECDSA pair.
fn curve(name: &str) -> Option<(&'static agreement::Algorithm, Option<Ecdsa>)> {
    use signature as s;
    Some(match name {
        "P-256" => (
            &agreement::ECDH_P256,
            Some((
                &s::ECDSA_P256_SHA256_FIXED_SIGNING,
                &s::ECDSA_P256_SHA256_FIXED,
                &digest::SHA256,
            )),
        ),
        "P-384" => (
            &agreement::ECDH_P384,
            Some((
                &s::ECDSA_P384_SHA384_FIXED_SIGNING,
                &s::ECDSA_P384_SHA384_FIXED,
                &digest::SHA384,
            )),
        ),
        "P-521" => (
            &agreement::ECDH_P521,
            Some((
                &s::ECDSA_P521_SHA512_FIXED_SIGNING,
                &s::ECDSA_P521_SHA512_FIXED,
                &digest::SHA512,
            )),
        ),
        "X25519" => (&agreement::X25519, None),
        _ => return None,
    })
}

/// A fresh key pair: `[private, public]`. Ed25519's private half is its seed.
pub fn generate(name: &str) -> Option<[Vec<u8>; 2]> {
    if name == "Ed25519" {
        return Ed25519KeyPair::generate().ok().map(|pair| {
            let seed = pair
                .seed()
                .and_then(|s| s.as_be_bytes())
                .map(|b| b.as_ref().to_vec());
            [
                seed.unwrap_or_default(),
                pair.public_key().as_ref().to_vec(),
            ]
        });
    }
    PrivateKey::generate(curve(name)?.0).ok().map(|key| {
        let private = if name == "X25519" {
            AsBigEndian::<Curve25519SeedBin>::as_be_bytes(&key).map(|b| b.as_ref().to_vec())
        } else {
            AsBigEndian::<EcPrivateKeyBin>::as_be_bytes(&key).map(|b| b.as_ref().to_vec())
        };
        [private.unwrap_or_default(), public_of(&key)]
    })
}

/// The public half of a private one — the check that a scalar or seed is a
/// key at all, and what an OKP JWK's `x` must equal.
pub fn public(name: &str, private: &[u8]) -> Option<Vec<u8>> {
    if name == "Ed25519" {
        return Ed25519KeyPair::from_seed_unchecked(private)
            .ok()
            .map(|pair| pair.public_key().as_ref().to_vec());
    }
    PrivateKey::from_private_key(curve(name)?.0, private)
        .ok()
        .map(|key| public_of(&key))
}

fn public_of(key: &PrivateKey) -> Vec<u8> {
    key.compute_public_key()
        .map(|p| p.as_ref().to_vec())
        .unwrap_or_default()
}

/// A public key of the curve as WebCrypto keeps it — a compressed NIST
/// point made uncompressed — or `None` when it is not one.
pub fn point(name: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    let nist = match name {
        "P-256" => Some(point::P256),
        "P-384" => Some(point::P384),
        "P-521" => Some(point::P521),
        _ => None,
    };
    let compressed = nist.filter(|c| bytes.len() == c.0.len() / 2 + 1 && matches!(bytes[0], 2 | 3));
    let whole = match compressed {
        Some(c) => point::decompress(c, bytes)?,
        None => bytes.to_vec(),
    };
    valid(name, &whole).then_some(whole)
}

/// Whether `point` is a public key of the curve: on it, and for X25519 not
/// of small order. An Ed25519 key is any 32 bytes until a verify decodes it.
fn valid(name: &str, point: &[u8]) -> bool {
    if name == "Ed25519" {
        return point.len() == 32;
    }
    curve(name).is_some_and(|(alg, _)| {
        ParsedPublicKey::try_from(&UnparsedPublicKey::new(alg, point)).is_ok()
            && (name != "X25519" || agree_with_any(alg, point))
    })
}

/// X25519 refuses a small-order point only when it agrees on it (the shared
/// secret would be all zeros): try it once with a fixed scalar.
fn agree_with_any(alg: &'static agreement::Algorithm, point: &[u8]) -> bool {
    PrivateKey::from_private_key(alg, &[9; 32]).is_ok_and(|key| {
        agreement::agree(&key, UnparsedPublicKey::new(alg, point), (), |_| Ok(())).is_ok()
    })
}

/// ECDSA over `hash`, any of the four with any curve, as WebCrypto allows.
/// The provider signs only its own curve/hash pairs, but ECDSA reads a digest
/// as the integer of its leftmost curve-order bits, so a longer digest is cut
/// and a shorter one zero-filled on the left into the pair's own length — the
/// same integer, the same signature (SEC 1 §4.1.3 step 5). The key is the
/// scalar alone: a page may pair it with any point (measured), and signs with
/// the scalar it gave.
pub fn ecdsa_sign(name: &str, hash: &str, d: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let (signing, _, to) = curve(name)?.1?;
    // RFC 5915 `ECPrivateKey` holding only the scalar: SEQUENCE { 1, d }.
    let n = d.len() as u8;
    let mut der = vec![0x30, n.wrapping_add(5), 2, 1, 1, 4, n];
    der.extend_from_slice(d);
    let pair = EcdsaKeyPair::from_private_key_der(signing, &der).ok()?;
    let digest = fitted(hash, data, to)?;
    pair.sign_digest(&digest).ok().map(|s| s.as_ref().to_vec())
}

pub fn ecdsa_verify(name: &str, hash: &str, point: &[u8], sig: &[u8], data: &[u8]) -> bool {
    curve(name)
        .and_then(|(_, ecdsa)| ecdsa)
        .and_then(|(_, verifying, to)| Some((verifying, fitted(hash, data, to)?)))
        .is_some_and(|(verifying, digest)| {
            signature::UnparsedPublicKey::new(verifying, point)
                .verify_digest(&digest, sig)
                .is_ok()
        })
}

fn fitted(hash: &str, data: &[u8], to: &'static digest::Algorithm) -> Option<Digest> {
    let made = digest::digest(super::alg::digest_algorithm(hash)?, data);
    let (h, n) = (made.as_ref(), to.output_len());
    let mut fit = vec![0; n.saturating_sub(h.len())];
    fit.extend_from_slice(&h[..h.len().min(n)]);
    Digest::import_less_safe(&fit, to).ok()
}

pub fn ed25519_sign(seed: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    Ed25519KeyPair::from_seed_unchecked(seed)
        .ok()
        .map(|pair| pair.sign(data).as_ref().to_vec())
}

pub fn ed25519_verify(public: &[u8], sig: &[u8], data: &[u8]) -> bool {
    signature::UnparsedPublicKey::new(&signature::ED25519, public)
        .verify(data, sig)
        .is_ok()
}

/// The shared secret of ECDH or X25519: the x-coordinate, or the 32 bytes.
pub fn agree(name: &str, private: &[u8], peer: &[u8]) -> Option<Vec<u8>> {
    let alg = curve(name)?.0;
    let key = PrivateKey::from_private_key(alg, private).ok()?;
    agreement::agree(&key, UnparsedPublicKey::new(alg, peer), (), |k| {
        Ok(k.to_vec())
    })
    .ok()
}
