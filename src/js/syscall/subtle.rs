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
