//! The `<img>` size syscall (`bl-91a4`, js.md §7 "Images").
//!
//! An image frot can *have* is one whose bytes are in the page — a `data:` URL
//! — or, under `--images` (`bl-49f1`), one the image wave fetched
//! (`subfetch/image.rs`). `__frot_image_size(url)` reads its intrinsic size —
//! `[width, height]`, or `null` for anything that is not a whole image frot
//! can size (an unfetched or failed URL, a non-image, a truncated file). The
//! prelude turns `null` into an `error` event and a size into a `load`
//! (`prelude/imageload.js`). `__frot_image_fetch(urls, backgrounds)`,
//! bound only under `--images`, is the wave.
//!
//! Hand-written header readers, no decoder dependency. The raster formats are
//! told apart by their signature, whatever type the URL declares — Firefox
//! sniffs (measured: a PNG labelled `text/plain` loads). SVG has no signature,
//! so it is SVG only when declared `image/svg+xml`, again as Firefox does.
//!
//! "Whole" is judged by the container, not by decoding: a PNG must end in its
//! `IEND` chunk, a GIF in its `;` trailer, a JPEG must carry `EOI` after its
//! frame header, a RIFF its declared length. Firefox fails a header-only file
//! (measured on all three raster formats), and these checks reproduce that
//! without decoding a pixel. A file that is whole but corrupt inside loads
//! here and fails in Firefox — the declared residual.

use rquickjs::{Ctx, Function, Object};

use super::Host;
use crate::js::subfetch::{data, SharedSubfetch};

mod avif;
mod svg;

/// Register `__frot_image_size(url) -> [w, h] | null` and, under `--images`
/// only, `__frot_image_fetch(requests)`.
pub fn install<'js>(ctx: &Ctx<'js>, g: &Object<'js>, host: &Host) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_image_size", {
        let sf = host.subfetch.clone();
        move |url: String| sized(&sf, &url).map(|(w, h)| vec![w, h])
    });
    if host.env.images {
        bind!(ctx, g, "__frot_image_fetch", {
            let sf = host.subfetch.clone();
            move |urls: Vec<String>, background: Vec<bool>| {
                let reqs: Vec<_> = urls.into_iter().zip(background).collect();
                sf.borrow_mut().fetch_images(&reqs)
            }
        });
    }
    Ok(())
}

/// The intrinsic size of the image a resolved `src` names, if frot has it: a
/// `data:` URL from its own bytes, anything else from the image wave.
fn sized(sf: &SharedSubfetch, url: &str) -> Option<(u32, u32)> {
    match data::payload(url) {
        Some(payload) => data::bytes(payload).ok().and_then(|(m, b)| size(m, &b)),
        None => sf.borrow().image(url),
    }
}

/// The size of image `bytes` declared as `mime`: raster formats by signature
/// whatever the declared type (Firefox sniffs), SVG only when declared.
pub(crate) fn size(mime: &str, bytes: &[u8]) -> Option<(u32, u32)> {
    let essence = mime.split(';').next().unwrap_or_default().trim();
    if essence.eq_ignore_ascii_case("image/svg+xml") {
        return svg::size(std::str::from_utf8(bytes).ok()?);
    }
    raster(bytes)
}

/// A raster image's size, by signature.
fn raster(b: &[u8]) -> Option<(u32, u32)> {
    match b {
        [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', ..] => png(b),
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => gif(b),
        [0xff, 0xd8, ..] => jpeg(b),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => webp(b),
        [b'B', b'M', ..] => bmp(b),
        [0, 0, 1, 0, ..] => ico(b),
        [_, _, _, _, b'f', b't', b'y', b'p', ..] => avif::size(b),
        _ => None,
    }
}

// Readers over a window already taken with one bounds-checked `get`, so no
// conversion below can fail and none needs a `?` of its own.
fn be16(s: &[u8]) -> u32 {
    u32::from(s[0]) << 8 | u32::from(s[1])
}

fn le16(s: &[u8]) -> u32 {
    u32::from(s[1]) << 8 | u32::from(s[0])
}

fn be32(s: &[u8]) -> u32 {
    be16(s) << 16 | be16(&s[2..])
}

fn le32(s: &[u8]) -> u32 {
    le16(&s[2..]) << 16 | le16(s)
}

/// IHDR is the first chunk; `IEND` (empty, so its 12 bytes are fixed) ends it.
fn png(b: &[u8]) -> Option<(u32, u32)> {
    let h = b.get(12..24).filter(|h| &h[..4] == b"IHDR")?;
    let whole = b.ends_with(b"\0\0\0\0IEND\xaeB`\x82");
    whole.then_some((be32(&h[4..]), be32(&h[8..])))
}

/// The logical screen size, little-endian at 6; `;` is the trailer.
fn gif(b: &[u8]) -> Option<(u32, u32)> {
    let h = b.get(6..10)?;
    (b.last() == Some(&b';')).then_some((le16(h), le16(&h[2..])))
}

/// Walk the marker segments to the first start-of-frame (`SOFn`, skipping the
/// three `C4`/`C8`/`CC` codes that share its range); `EOI` must follow it. In
/// entropy-coded data `FF` is stuffed as `FF 00` and restarts are `D0`–`D7`,
/// so a later `FF D9` can only be the real end. Every step moves forward, so a
/// malformed file ends the walk at its last byte.
fn jpeg(b: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    loop {
        let seg = b.get(at..at + 4).filter(|s| s[0] == 0xff)?;
        if matches!(seg[1], 0xc0..=0xcf) && !matches!(seg[1], 0xc4 | 0xc8 | 0xcc) {
            let f = b.get(at + 5..at + 9)?;
            let whole = b[at..].windows(2).any(|m| m == [0xff, 0xd9]);
            return whole.then_some((be16(&f[2..]), be16(f)));
        }
        at += 2 + be16(&seg[2..]) as usize;
    }
}

/// The first chunk names the bitstream: `VP8 ` (lossy, 14-bit sizes after the
/// frame tag), `VP8L` (lossless, packed `size - 1`), `VP8X` (extended, 24-bit
/// `size - 1`). The RIFF header's length must fit in what is here. The
/// signature match guarantees the first 12 bytes.
fn webp(b: &[u8]) -> Option<(u32, u32)> {
    let whole = le32(&b[4..]) as usize + 8 <= b.len();
    let size = match b.get(12..16)? {
        b"VP8 " => b
            .get(26..30)
            .map(|f| (le16(f) & 0x3fff, le16(&f[2..]) & 0x3fff))?,
        b"VP8L" => b.get(21..25).map(|f| {
            let bits = le32(f);
            (1 + (bits & 0x3fff), 1 + ((bits >> 14) & 0x3fff))
        })?,
        b"VP8X" => b
            .get(24..31)
            .map(|f| (1 + (le32(f) & 0xff_ffff), 1 + (le32(&f[3..]) & 0xff_ffff)))?,
        _ => return None,
    };
    whole.then_some(size)
}

/// `BITMAPINFOHEADER` sizes at 18/22 (a negative height is a top-down bitmap);
/// the file header's length must fit.
fn bmp(b: &[u8]) -> Option<(u32, u32)> {
    let h = b.get(2..26)?;
    let whole = le32(h) as usize <= b.len();
    let side = |s: &[u8]| (le32(s) as i32).unsigned_abs();
    whole.then_some((side(&h[16..]), side(&h[20..])))
}

/// The first directory entry's one-byte sizes, where 0 means 256; its image
/// data (size and offset at 8/12 of the entry) must fit.
fn ico(b: &[u8]) -> Option<(u32, u32)> {
    let e = b.get(6..22)?;
    let whole = le32(&e[8..]) as usize + le32(&e[12..]) as usize <= b.len();
    let side = |v: u8| if v == 0 { 256 } else { u32::from(v) };
    whole.then_some((side(e[0]), side(e[1])))
}

#[cfg(test)]
mod tests;
