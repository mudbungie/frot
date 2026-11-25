//! AVIF's intrinsic size (`bl-49f1`, js.md §7 "Images"): the persona's image
//! `Accept` advertises `image/avif`, so frot must size what it asks for.
//!
//! An AVIF file is ISOBMFF boxes: `ftyp` naming an `avif`/`avis` brand, then a
//! `meta` box whose `iprp`/`ipco` property container holds one `ispe` (image
//! spatial extents) per image item, then `mdat` with the pixels. The size is the
//! largest `ispe` — the primary image's in the single, alpha, grid and
//! thumbnail layouts, where every other item is the same size or smaller.
//! "Whole" is the container's: the top-level boxes must tile the file exactly
//! and include the `mdat` that holds the pixels, as Firefox fails a cut file. `irot`/`imir` are not applied (the declared
//! residual, as EXIF orientation is for JPEG).

use super::be32;

/// The size of a whole AVIF `b`, or `None`.
pub(super) fn size(b: &[u8]) -> Option<(u32, u32)> {
    let top = boxes(b)?;
    let brands = top.iter().find(|(t, _)| t == b"ftyp")?.1;
    let avif = brands.chunks_exact(4).any(|c| c == b"avif" || c == b"avis");
    let meta = top.iter().find(|(t, _)| t == b"meta")?.1.get(4..)?;
    let ipco = child(child(meta, b"iprp")?, b"ipco")?;
    let ispe = boxes(ipco)?
        .into_iter()
        .filter(|(t, _)| t == b"ispe")
        .filter_map(|(_, p)| p.get(4..12).map(|s| (be32(s), be32(&s[4..]))))
        .max_by_key(|&(w, h)| u64::from(w) * u64::from(h));
    let pixels = top.iter().any(|(t, _)| t == b"mdat");
    ispe.filter(|_| avif && pixels)
}

/// The payload of the first `kind` box directly inside `b`.
fn child<'a>(b: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
    boxes(b)?
        .into_iter()
        .find(|(t, _)| t == kind)
        .map(|(_, p)| p)
}

/// Split `b` into `(type, payload)` boxes, or `None` unless they tile it
/// exactly. A 32-bit size of 1 means a 64-bit size follows the type; 0 means
/// the box runs to the end.
fn boxes(mut b: &[u8]) -> Option<Vec<([u8; 4], &[u8])>> {
    let mut out = Vec::new();
    while !b.is_empty() {
        let head = b.get(..8)?;
        let kind = [head[4], head[5], head[6], head[7]];
        let (len, skip) = match be32(head) {
            0 => (b.len(), 8),
            1 => {
                let l = b.get(8..16)?;
                (
                    (u64::from(be32(l)) << 32 | u64::from(be32(&l[4..]))) as usize,
                    16,
                )
            }
            n => (n as usize, 8),
        };
        out.push((kind, b.get(skip..len)?));
        b = &b[len..];
    }
    Some(out)
}

#[cfg(test)]
mod tests;
