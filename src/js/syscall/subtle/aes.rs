//! AES in the four modes WebCrypto names (`bl-db8f`): CBC, CTR, GCM and KW, for
//! 128/192/256-bit keys. `None` means "not something this table computes" — a
//! key, IV or length outside the mode — or, for the three that can tell,
//! "did not decrypt": bad CBC padding, a GCM tag or a KW integrity check that
//! does not match. The prelude owns what a page is told about it.
//!
//! The block cipher, CBC and CTR are the provider's (`aws-lc-rs`). Two
//! constructions are built here on top of it, because the provider's own forms
//! are narrower than what Firefox 153 performs (identity.md §3.19):
//!
//! - **GCM** — the provider's AEAD fixes the nonce at 96 bits and the tag at
//!   128; Firefox takes any non-empty nonce and seven tag lengths. So GCM is
//!   CTR plus GHASH ([`gmul`]), one path for every size.
//! - **KW** (RFC 3394) — the provider has no 192-bit key wrap; Firefox does. It
//!   is twelve lines over the block cipher.
//!
//! GHASH here is bit-serial and not constant-time. The key it runs under is the
//! page's own, in the page's own process, and nothing here is reachable from
//! the TLS stack.

use aws_lc_rs::cipher::{
    self, DecryptingKey, DecryptionContext, EncryptingKey, EncryptionContext,
    PaddedBlockDecryptingKey, PaddedBlockEncryptingKey, UnboundCipherKey,
};

/// The provider key for 16, 24 or 32 bytes; `None` for any other length.
fn key(bytes: &[u8]) -> Option<UnboundCipherKey> {
    let alg = match bytes.len() {
        16 => &cipher::AES_128,
        24 => &cipher::AES_192,
        32 => &cipher::AES_256,
        _ => return None,
    };
    UnboundCipherKey::new(alg, bytes).ok()
}

/// PKCS#7-padded CBC — WebCrypto's AES-CBC. The IV must be one block.
pub fn cbc_seal(k: &[u8], iv: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let iv: [u8; 16] = iv.try_into().ok()?;
    let k = key(k).and_then(|k| PaddedBlockEncryptingKey::cbc_pkcs7(k).ok())?;
    let mut buf = data.to_vec();
    let sealed = k.less_safe_encrypt(&mut buf, EncryptionContext::Iv128(iv.into()));
    sealed.ok().map(|_| buf)
}

/// The plaintext, or `None` if the ciphertext is not whole blocks or its
/// padding is malformed.
pub fn cbc_open(k: &[u8], iv: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let iv: [u8; 16] = iv.try_into().ok()?;
    let k = key(k).and_then(|k| PaddedBlockDecryptingKey::cbc_pkcs7(k).ok())?;
    let mut buf = data.to_vec();
    let len = k
        .decrypt(&mut buf, DecryptionContext::Iv128(iv.into()))
        .ok()?
        .len();
    buf.truncate(len);
    Some(buf)
}

/// A CTR keystream over `data` from the counter block `block`, counting in
/// the bits `mask` selects: it wraps *within* them and never carries into the
/// nonce above. The provider counts across all 128 bits, so the stream is cut
/// at each wrap and restarted with the counter bits cleared.
fn stream(k: &EncryptingKey, mut block: u128, mask: u128, data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    let mut rest = &mut out[..];
    while !rest.is_empty() {
        let blocks = (mask - (block & mask)).saturating_add(1);
        let take = usize::try_from(blocks.saturating_mul(16)).unwrap_or(usize::MAX);
        let (run, tail) = rest.split_at_mut(take.min(rest.len()));
        k.less_safe_encrypt(run, EncryptionContext::Iv128(block.to_be_bytes().into()))
            .expect("AES-CTR over a valid key and counter block");
        block &= !mask;
        rest = tail;
    }
    out
}

/// CTR whose counter is the low `bits` bits (1 to 128) of the 16-byte
/// `counter` block — WebCrypto's `AesCtrParams.length`. Encrypts and decrypts
/// alike.
pub fn ctr(k: &[u8], counter: &[u8], bits: u32, data: &[u8]) -> Option<Vec<u8>> {
    let block = u128::from_be_bytes(counter.try_into().ok()?);
    let mask = 128u32
        .checked_sub(bits)
        .and_then(|shift| u128::MAX.checked_shr(shift))?;
    let k = key(k).and_then(|k| EncryptingKey::ctr(k).ok())?;
    Some(stream(&k, block, mask, data))
}

/// One block through the cipher itself (ECB), forwards.
fn forward(k: &EncryptingKey, block: u128) -> u128 {
    let mut b = block.to_be_bytes();
    k.less_safe_encrypt(&mut b, EncryptionContext::None)
        .expect("AES over one whole block");
    u128::from_be_bytes(b)
}

/// One block through the cipher, backwards.
fn backward(k: &DecryptingKey, block: u128) -> u128 {
    let mut b = block.to_be_bytes();
    k.decrypt(&mut b, DecryptionContext::None)
        .expect("AES over one whole block");
    u128::from_be_bytes(b)
}

/// Multiplication in GF(2^128) as GCM defines it (bit 0 is the most significant;
/// the reduction polynomial is `0xe1 << 120`).
fn gmul(x: u128, h: u128) -> u128 {
    let (mut z, mut v) = (0u128, h);
    for i in 0..128 {
        z ^= v * ((x >> (127 - i)) & 1);
        v = (v >> 1) ^ ((0xe1 << 120) * (v & 1));
    }
    z
}

/// GHASH over `parts`, each zero-padded to whole blocks.
fn ghash(h: u128, parts: &[&[u8]]) -> u128 {
    let blocks = parts.iter().flat_map(|part| part.chunks(16));
    blocks.fold(0, |y, chunk| {
        let mut b = [0u8; 16];
        b[..chunk.len()].copy_from_slice(chunk);
        gmul(y ^ u128::from_be_bytes(b), h)
    })
}

/// The bit length of `bytes` as GCM's length fields carry it.
fn bits(bytes: &[u8]) -> u128 {
    bytes.len() as u128 * 8
}

/// What GCM derives from a key and nonce.
struct Gcm {
    /// The hash subkey.
    h: u128,
    /// The keystream block that masks the tag: the pre-counter block `J0`,
    /// encrypted.
    mask: u128,
    /// The counter block the message keystream starts at: `J0` plus one.
    first: u128,
    ctr: EncryptingKey,
}

impl Gcm {
    fn new(k: &[u8], iv: &[u8]) -> Option<Self> {
        let ecb = key(k).and_then(|k| EncryptingKey::ecb(k).ok());
        let ctr = key(k).and_then(|k| EncryptingKey::ctr(k).ok());
        let (ecb, ctr) = ecb.zip(ctr)?;
        let h = forward(&ecb, 0);
        let mut nonce = [0u8; 16];
        let j0 = if iv.len() == 12 {
            nonce[..12].copy_from_slice(iv);
            u128::from_be_bytes(nonce) | 1
        } else {
            ghash(h, &[iv, &bits(iv).to_be_bytes()])
        };
        Some(Self {
            h,
            mask: forward(&ecb, j0),
            first: (j0 & !COUNTER) | (j0.wrapping_add(1) & COUNTER),
            ctr,
        })
    }

    /// The keystream applied: GCM counts in the low 32 bits only.
    fn crypt(&self, data: &[u8]) -> Vec<u8> {
        stream(&self.ctr, self.first, COUNTER, data)
    }

    /// The full 16-byte tag over associated data and ciphertext.
    fn tag(&self, aad: &[u8], sealed: &[u8]) -> [u8; 16] {
        let lengths = (bits(aad) << 64) | bits(sealed);
        (ghash(self.h, &[aad, sealed, &lengths.to_be_bytes()]) ^ self.mask).to_be_bytes()
    }
}

/// The bits of a GCM counter block that count.
const COUNTER: u128 = u32::MAX as u128;

/// Ciphertext with the first `tag_len` bytes of the tag appended — WebCrypto's
/// layout. `None` for a `tag_len` over 16.
pub fn gcm_seal(k: &[u8], iv: &[u8], aad: &[u8], tag_len: usize, data: &[u8]) -> Option<Vec<u8>> {
    let gcm = Gcm::new(k, iv)?;
    let mut out = gcm.crypt(data);
    let tag = gcm.tag(aad, &out);
    out.extend_from_slice(tag.get(..tag_len)?);
    Some(out)
}

/// The plaintext of `data` (ciphertext + a `tag_len`-byte tag), or `None` if it
/// does not authenticate under this key, nonce and associated data.
pub fn gcm_open(k: &[u8], iv: &[u8], aad: &[u8], tag_len: usize, data: &[u8]) -> Option<Vec<u8>> {
    let gcm = Gcm::new(k, iv)?;
    let (sealed, theirs) = data.split_at(data.len().checked_sub(tag_len)?);
    let ours = gcm.tag(aad, sealed);
    let diff = ours.get(..tag_len)?.iter().zip(theirs);
    (diff.fold(0, |acc, (a, b)| acc | (a ^ b)) == 0).then(|| gcm.crypt(sealed))
}

/// RFC 3394's initial value: the integrity check an unwrap must recover.
const KW_IV: u64 = 0xa6a6_a6a6_a6a6_a6a6;

/// `data` as 64-bit words — KW's unit — or `None` unless it is at least
/// `least` whole words.
fn words(data: &[u8], least: usize) -> Option<Vec<u64>> {
    let whole = data.len().is_multiple_of(8) && data.len() / 8 >= least;
    let word = |w: &[u8]| u64::from_be_bytes(w.try_into().expect("an 8-byte chunk"));
    whole.then(|| data.chunks(8).map(word).collect())
}

fn unwords(words: &[u64]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_be_bytes()).collect()
}

/// RFC 3394 key wrap: `data` (two or more 64-bit words) grows by one word.
pub fn kw_wrap(k: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let k = key(k).and_then(|k| EncryptingKey::ecb(k).ok())?;
    let mut r = words(data, 2)?;
    let mut a = KW_IV;
    for t in 1..=6 * r.len() as u64 {
        let i = (t as usize - 1) % r.len();
        let b = forward(&k, (u128::from(a) << 64) | u128::from(r[i]));
        a = (b >> 64) as u64 ^ t;
        r[i] = b as u64;
    }
    r.insert(0, a);
    Some(unwords(&r))
}

/// The wrapped key, or `None` if the integrity check does not come out.
pub fn kw_unwrap(k: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    let k = key(k).and_then(|k| DecryptingKey::ecb(k).ok())?;
    let mut r = words(data, 3)?;
    let mut a = r.remove(0);
    for t in (1..=6 * r.len() as u64).rev() {
        let i = (t as usize - 1) % r.len();
        let b = backward(&k, (u128::from(a ^ t) << 64) | u128::from(r[i]));
        a = (b >> 64) as u64;
        r[i] = b as u64;
    }
    (a == KW_IV).then(|| unwords(&r))
}

#[cfg(test)]
mod tests;
