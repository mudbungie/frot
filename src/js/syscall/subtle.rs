//! The WebCrypto primitive syscalls (`bl-0647`, `bl-db8f`; identity.md
//! §3.19/§11) — the host operations behind `crypto.subtle`.
//!
//! A page *awaits and consumes* what `SubtleCrypto` returns — AWS WAF's
//! challenge encrypts its answer with it (`challenge.md` §11.4) — so these are
//! the real primitives, computed by the crypto provider the TLS stack already
//! links, never a costume. Everything WebCrypto-shaped (algorithm
//! normalization, `CryptoKey`, usages, the error names, the promises) is the
//! `subtle.js` prelude's; this table only computes.
//!
//! Each call takes its bytes as `Uint8Array`s and returns a fresh `Uint8Array`,
//! or `undefined` when the operation cannot be performed (an algorithm name or
//! key/nonce size outside [`alg`] and [`aes`], or a ciphertext that does not
//! decrypt or authenticate) — which
//! the prelude turns into the DOMException a browser rejects with. Nothing is
//! kept between calls: a key is the page's bytes, passed in again each time.

use rquickjs::{Ctx, Function, Object, TypedArray};

use super::Host;

mod aes;
mod alg;
mod pk;
mod rsa;

macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

type Bytes<'js> = TypedArray<'js, u8>;

/// Register the `__frot_subtle_*` table: `digest(hash, data)`, `hmac(hash, key,
/// data)`, `pbkdf2(hash, key, salt, rounds, len)`, `hkdf(hash, key, salt, info,
/// len)`, and the AES modes — `cbc(open, key, iv, data)`, `ctr(key, counter,
/// bits, data)`, `gcm(open, key, iv, aad, tag_len, data)`, `kw(open, key,
/// data)`. `open` selects decryption; CTR is its own inverse.
pub fn install<'js>(ctx: &Ctx<'js>, g: &Object<'js>, _host: &Host) -> rquickjs::Result<()> {
    bind!(
        ctx,
        g,
        "__frot_subtle_digest",
        |ctx: Ctx<'js>, hash: String, data: Bytes<'js>| out(ctx, alg::digest(&hash, bytes(&data)))
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_hmac",
        |ctx: Ctx<'js>, hash: String, key: Bytes<'js>, data: Bytes<'js>| {
            out(ctx, alg::hmac(&hash, bytes(&key), bytes(&data)))
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_pbkdf2",
        |ctx: Ctx<'js>, hash: String, key: Bytes<'js>, salt: Bytes<'js>, rounds: u32, len: u32| {
            let made = alg::pbkdf2(&hash, bytes(&key), bytes(&salt), rounds, len as usize);
            out(ctx, made)
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_hkdf",
        |ctx: Ctx<'js>,
         hash: String,
         key: Bytes<'js>,
         salt: Bytes<'js>,
         info: Bytes<'js>,
         len: u32| {
            let made = alg::hkdf(&hash, bytes(&key), bytes(&salt), bytes(&info), len as usize);
            out(ctx, made)
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_cbc",
        |ctx: Ctx<'js>, open: bool, key: Bytes<'js>, iv: Bytes<'js>, data: Bytes<'js>| {
            let run = if open { aes::cbc_open } else { aes::cbc_seal };
            out(ctx, run(bytes(&key), bytes(&iv), bytes(&data)))
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_ctr",
        |ctx: Ctx<'js>, key: Bytes<'js>, counter: Bytes<'js>, bits: u32, data: Bytes<'js>| {
            out(
                ctx,
                aes::ctr(bytes(&key), bytes(&counter), bits, bytes(&data)),
            )
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_gcm",
        |ctx: Ctx<'js>,
         open: bool,
         key: Bytes<'js>,
         iv: Bytes<'js>,
         aad: Bytes<'js>,
         tag_len: u32,
         data: Bytes<'js>| {
            let run = if open { aes::gcm_open } else { aes::gcm_seal };
            let (key, iv, aad) = (bytes(&key), bytes(&iv), bytes(&aad));
            out(ctx, run(key, iv, aad, tag_len as usize, bytes(&data)))
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_kw",
        |ctx: Ctx<'js>, open: bool, key: Bytes<'js>, data: Bytes<'js>| {
            let run = if open { aes::kw_unwrap } else { aes::kw_wrap };
            out(ctx, run(bytes(&key), bytes(&data)))
        }
    );
    public_key(ctx, g)?;
    rsa_table(ctx, g)
}

/// The RSA table over [`rsa`] (`bl-2763`): `rsa_generate(bits)` answers
/// `[pkcs8, spki]`; `rsa_check(private, der)`, `rsa_sign(pss, hash, pkcs8,
/// data)`, `rsa_verify(pss, hash, spki, sig, data)`, `rsa_oaep(open, hash, der,
/// label, data)`.
fn rsa_table<'js>(ctx: &Ctx<'js>, g: &Object<'js>) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_subtle_rsa_generate", |bits: u32| {
        rsa::generate(bits).map(Vec::from)
    });
    bind!(
        ctx,
        g,
        "__frot_subtle_rsa_check",
        |private: bool, der: Bytes<'js>| { rsa::check(private, bytes(&der)) }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_rsa_sign",
        |ctx: Ctx<'js>, pss: bool, h: String, der: Bytes<'js>, data: Bytes<'js>| {
            out(ctx, rsa::sign(pss, &h, bytes(&der), bytes(&data)))
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_rsa_verify",
        |pss: bool, h: String, der: Bytes<'js>, sig: Bytes<'js>, data: Bytes<'js>| {
            rsa::verify(pss, &h, bytes(&der), bytes(&sig), bytes(&data))
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_rsa_oaep",
        |ctx: Ctx<'js>,
         open: bool,
         h: String,
         der: Bytes<'js>,
         label: Bytes<'js>,
         data: Bytes<'js>| {
            let (der, label) = (bytes(&der), bytes(&label));
            out(ctx, rsa::crypt(open, &h, der, label, bytes(&data)))
        }
    );
    Ok(())
}

/// The public-key table over [`pk`] (`bl-0cae`): `pk_generate(curve)` answers
/// `[private, public]` (as two arrays of bytes); `pk_public(curve, private)`, `pk_point(curve, bytes)` (the point uncompressed, if it is one);
/// `ecdsa_sign(curve, hash, d, data)`, `ecdsa_verify(curve, hash, point, sig, data)`, `ed25519_sign(seed,
/// data)`, `ed25519_verify(public, sig, data)`, `agree(curve, private, peer)`.
fn public_key<'js>(ctx: &Ctx<'js>, g: &Object<'js>) -> rquickjs::Result<()> {
    // Two arrays of bytes, which the prelude makes `Uint8Array`s.
    bind!(ctx, g, "__frot_subtle_pk_generate", |c: String| {
        pk::generate(&c).map(Vec::from)
    });
    bind!(
        ctx,
        g,
        "__frot_subtle_pk_public",
        |ctx: Ctx<'js>, c: String, private: Bytes<'js>| out(ctx, pk::public(&c, bytes(&private)))
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_pk_point",
        |ctx: Ctx<'js>, c: String, p: Bytes<'js>| out(ctx, pk::point(&c, bytes(&p)))
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_ecdsa_sign",
        |ctx: Ctx<'js>, c: String, h: String, d: Bytes<'js>, data: Bytes<'js>| {
            out(ctx, pk::ecdsa_sign(&c, &h, bytes(&d), bytes(&data)))
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_ecdsa_verify",
        |c: String, h: String, p: Bytes<'js>, sig: Bytes<'js>, data: Bytes<'js>| {
            pk::ecdsa_verify(&c, &h, bytes(&p), bytes(&sig), bytes(&data))
        }
    );
    bind!(
        ctx,
        g,
        "__frot_subtle_ed25519_sign",
        |ctx: Ctx<'js>, seed: Bytes<'js>, data: Bytes<'js>| {
            out(ctx, pk::ed25519_sign(bytes(&seed), bytes(&data)))
        }
    );
    bind!(ctx, g, "__frot_subtle_ed25519_verify", |p: Bytes<'js>,
                                                   sig: Bytes<'js>,
                                                   data: Bytes<
        'js,
    >| {
        pk::ed25519_verify(bytes(&p), bytes(&sig), bytes(&data))
    });
    bind!(
        ctx,
        g,
        "__frot_subtle_agree",
        |ctx: Ctx<'js>, c: String, private: Bytes<'js>, peer: Bytes<'js>| {
            out(ctx, pk::agree(&c, bytes(&private), bytes(&peer)))
        }
    );
    Ok(())
}

/// The view's bytes. A detached buffer has none, and reads as empty — what the
/// same view reports to the page (`byteLength` 0).
fn bytes<'a>(view: &'a Bytes<'_>) -> &'a [u8] {
    view.as_bytes().unwrap_or(&[])
}

/// A result as a fresh `Uint8Array`; `None` is `undefined`.
fn out<'js>(ctx: Ctx<'js>, result: Option<Vec<u8>>) -> rquickjs::Result<Option<Bytes<'js>>> {
    result.map(|v| TypedArray::new(ctx, v)).transpose()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod pk_tests;
#[cfg(test)]
mod rsa_tests;
