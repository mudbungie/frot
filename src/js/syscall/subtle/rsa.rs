//! RSA under WebCrypto (`bl-2763`): RSASSA-PKCS1-v1_5, RSA-PSS and RSA-OAEP,
//! as far as the provider computes them. A key crosses this boundary as DER —
//! PKCS #8 for a private key, `SubjectPublicKeyInfo` for a public one — and
//! nothing is held between calls; the JWK members, the containers' layout and
//! every WebCrypto rule are the prelude's. What the provider cannot do — a
//! key outside 2048–8192 bits, signing with SHA-1, PSS with SHA-1, generating
//! with another exponent or size — the prelude refuses before it gets here.

use aws_lc_rs::encoding::AsDer;
use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::rsa::{
    KeyPair, KeySize, OaepAlgorithm, OaepPrivateDecryptingKey, OaepPublicEncryptingKey,
    PrivateDecryptingKey, PublicEncryptingKey, OAEP_SHA1_MGF1SHA1, OAEP_SHA256_MGF1SHA256,
    OAEP_SHA384_MGF1SHA384, OAEP_SHA512_MGF1SHA512,
};
use aws_lc_rs::signature::{self as s, KeyPair as _, RsaEncoding, RsaParameters, RsaPssSaltLength};

/// A fresh key of `bits` — 2048, 3072 or 4096; 8192 is left out, its search
/// runs past the compute budget — with exponent 65537: `[pkcs8, spki]`.
pub fn generate(bits: u32) -> Option<[Vec<u8>; 2]> {
    let size = match bits {
        2048 => KeySize::Rsa2048,
        3072 => KeySize::Rsa3072,
        4096 => KeySize::Rsa4096,
        _ => return None,
    };
    let pair = KeyPair::generate(size).expect("RSA key generation");
    let pkcs8 = pair.as_der().expect("PKCS #8 of a fresh key");
    let spki = pair.public_key().as_der().expect("SPKI of a fresh key");
    Some([pkcs8.as_ref().to_vec(), spki.as_ref().to_vec()])
}

/// Whether the provider takes this key: `1` it does, `-1` its size is outside
/// 2048–8192 bits, `0` it is not a key.
pub fn check(private: bool, der: &[u8]) -> i32 {
    let taken = match private {
        true => KeyPair::from_pkcs8(der).map(drop),
        false => PublicEncryptingKey::from_der(der).map(drop),
    };
    match taken {
        Ok(()) => 1,
        Err(e) if matches!(e.description_(), "TooSmall" | "TooLarge") => -1,
        Err(_) => 0,
    }
}

/// The signing encoding and verification parameters of `hash` under PKCS #1
/// v1.5 (`salt` negative) or under PSS salted with `salt` bytes (`bl-4ba0`).
/// The provider takes only `'static` ones, so a PSS pair is leaked: a few
/// dozen bytes per call, for the life of the one-shot process.
fn scheme(
    salt: i32,
    hash: &str,
) -> Option<(Option<&'static dyn RsaEncoding>, &'static RsaParameters)> {
    if salt < 0 {
        return Some(match hash {
            "SHA-1" => (None, &s::RSA_PKCS1_2048_8192_SHA1_FOR_LEGACY_USE_ONLY),
            "SHA-256" => (Some(&s::RSA_PKCS1_SHA256), &s::RSA_PKCS1_2048_8192_SHA256),
            "SHA-384" => (Some(&s::RSA_PKCS1_SHA384), &s::RSA_PKCS1_2048_8192_SHA384),
            "SHA-512" => (Some(&s::RSA_PKCS1_SHA512), &s::RSA_PKCS1_2048_8192_SHA512),
            _ => return None,
        });
    }
    let len = RsaPssSaltLength::Custom(u16::try_from(salt).ok()?);
    let (encoding, params) = match hash {
        "SHA-256" => (s::RSA_PSS_SHA256, s::RSA_PSS_2048_8192_SHA256),
        "SHA-384" => (s::RSA_PSS_SHA384, s::RSA_PSS_2048_8192_SHA384),
        "SHA-512" => (s::RSA_PSS_SHA512, s::RSA_PSS_2048_8192_SHA512),
        _ => return None,
    };
    let encoding: &'static dyn RsaEncoding =
        Box::leak(Box::new(encoding.with_pss_salt_length(len)));
    Some((
        Some(encoding),
        Box::leak(Box::new(params.with_pss_salt_length(len))),
    ))
}

/// A PKCS #1 v1.5 signature (`salt` negative) or a PSS one salted with `salt`.
pub fn sign(salt: i32, hash: &str, pkcs8: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let encoding = scheme(salt, hash)?.0?;
    let pair = KeyPair::from_pkcs8(pkcs8).ok()?;
    let mut sig = vec![0; pair.public_modulus_len()];
    pair.sign(encoding, &SystemRandom::new(), data, &mut sig)
        .expect("RSA signature");
    Some(sig)
}

pub fn verify(salt: i32, hash: &str, spki: &[u8], sig: &[u8], data: &[u8]) -> bool {
    scheme(salt, hash).is_some_and(|(_, params)| {
        s::UnparsedPublicKey::new(params, spki)
            .verify(data, sig)
            .is_ok()
    })
}

fn oaep(hash: &str) -> Option<&'static OaepAlgorithm> {
    Some(match hash {
        "SHA-1" => &OAEP_SHA1_MGF1SHA1,
        "SHA-256" => &OAEP_SHA256_MGF1SHA256,
        "SHA-384" => &OAEP_SHA384_MGF1SHA384,
        "SHA-512" => &OAEP_SHA512_MGF1SHA512,
        _ => return None,
    })
}

/// RSA-OAEP: `open` decrypts with a PKCS #8 key, else encrypts with an SPKI
/// one. `None` when the data does not fit or does not decrypt.
pub fn crypt(open: bool, hash: &str, der: &[u8], label: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let alg = oaep(hash)?;
    let label = Some(label);
    if open {
        let key = PrivateDecryptingKey::from_pkcs8(der).ok()?;
        let key = OaepPrivateDecryptingKey::new(key).expect("an RSA key decrypts");
        let mut out = vec![0; key.min_output_size()];
        let n = key.decrypt(alg, data, &mut out, label).ok()?.len();
        out.truncate(n);
        return Some(out);
    }
    let key = PublicEncryptingKey::from_der(der).ok()?;
    let key = OaepPublicEncryptingKey::new(key).expect("an RSA key encrypts");
    let mut out = vec![0; key.ciphertext_size()];
    key.encrypt(alg, data, &mut out, label)
        .ok()
        .map(|c| c.to_vec())
}
