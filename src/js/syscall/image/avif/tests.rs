//! The AVIF reader against a real file (libaom, 37×23, still picture) and
//! against the box layouts and cuts it must refuse.

use super::super::raster;
use super::size;

/// `ffmpeg -f lavfi -i color=c=red:s=37x23 -frames:v 1 -c:v libaom-av1
/// -still-picture 1 a.avif`.
const AVIF: &str = "AAAAIGZ0eXBhdmlmAAAAAGF2aWZtaWYxbWlhZk1BMUIAAAD5bWV0YQAAAAAAAAAvaGRscgAAAAAAAAAAcGljdAAAAAAAAAAAAAAAAFBpY3R1cmVIYW5kbGVyAAAAAA5waXRtAAAAAAABAAAAHmlsb2MAAAAARAAAAQABAAAAAQAAASEAAAAdAAAAKGlpbmYAAAAAAAEAAAAaaW5mZQIAAAAAAQAAYXYwMUNvbG9yAAAAAGppcHJwAAAAS2lwY28AAAAUaXNwZQAAAAAAAAAlAAAAFwAAABBwaXhpAAAAAAMICAgAAAAMYXYxQ4EADAAAAAATY29scm5jbHgAAgACAAIAAAAAF2lwbWEAAAAAAAAAAQABBAECgwQAAAAlbWRhdAoGGBUks2AQMhMYAAAAUAAAAAs8/LtEAS+2ZrSg";

fn raw() -> Vec<u8> {
    crate::js::subfetch::data::bytes(&format!("image/x;base64,{AVIF}"))
        .unwrap()
        .1
}

/// One box: a 32-bit size, the type, the payload.
fn bx(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut b = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    b.extend_from_slice(kind);
    b.extend_from_slice(payload);
    b
}

/// An `ispe` property: a full-box header, then width and height.
fn ispe(w: u32, h: u32) -> Vec<u8> {
    let mut p = vec![0; 4];
    p.extend_from_slice(&w.to_be_bytes());
    p.extend_from_slice(&h.to_be_bytes());
    bx(b"ispe", &p)
}

/// A minimal AVIF: `ftyp` with `brand`, `meta` holding `props`, then `mdat`.
fn file(brand: &[u8; 4], props: &[u8]) -> Vec<u8> {
    let mut ftyp = brand.to_vec();
    ftyp.extend_from_slice(&[0; 4]);
    ftyp.extend_from_slice(b"mif1");
    let ipco = bx(b"ipco", props);
    let mut meta = vec![0; 4];
    meta.extend(bx(b"iprp", &ipco));
    let mut f = bx(b"ftyp", &ftyp);
    f.extend(bx(b"meta", &meta));
    f.extend(bx(b"mdat", &[1, 2, 3]));
    f
}

#[test]
fn a_real_avif_is_sized_by_signature() {
    assert_eq!(raster(&raw()), Some((37, 23)));
}

#[test]
fn every_cut_of_a_real_avif_fails() {
    let b = raw();
    for n in 0..b.len() {
        assert_eq!(raster(&b[..n]), None, "{n} of {}", b.len());
    }
}

#[test]
fn the_largest_ispe_is_the_image() {
    // A grid's tiles, or a thumbnail, beside the full-size item.
    let props = [ispe(10, 10), ispe(640, 480), ispe(320, 240)].concat();
    assert_eq!(size(&file(b"avif", &props)), Some((640, 480)));
    // `avis` (an image sequence) is an AVIF brand too.
    assert_eq!(size(&file(b"avis", &ispe(5, 6))), Some((5, 6)));
}

#[test]
fn not_an_avif_brand_or_no_size_fails() {
    assert_eq!(size(&file(b"heic", &ispe(5, 6))), None);
    assert_eq!(size(&file(b"avif", &[])), None);
    // An `ispe` too short to hold both sides.
    assert_eq!(size(&file(b"avif", &bx(b"ispe", &[0; 8]))), None);
    // No `meta` at all, and a `meta` too short for its full-box header.
    let ftyp = bx(b"ftyp", b"avif\0\0\0\0");
    assert_eq!(size(&ftyp), None);
    assert_eq!(size(&[ftyp.clone(), bx(b"meta", &[0])].concat()), None);
    // Everything but the pixels: cut where `mdat` would start.
    let f = file(b"avif", &ispe(5, 6));
    assert_eq!(size(&f[..f.len() - 11]), None);
    // A `meta` with no `iprp`, and an `ftyp`-less file.
    assert_eq!(size(&[ftyp, bx(b"meta", &[0; 4])].concat()), None);
    assert_eq!(size(&bx(b"meta", &[0; 4])), None);
    // An `iprp` with no `ipco`; a `meta` and an `ipco` whose boxes do not tile.
    let ftyp = bx(b"ftyp", b"avif\0\0\0\0");
    let mdat = bx(b"mdat", &[1]);
    let meta = |inner: &[u8]| bx(b"meta", &[&[0; 4][..], inner].concat());
    for inner in [
        bx(b"iprp", &[]),
        vec![0, 0, 0, 9],
        bx(b"iprp", &[0, 0, 0, 9]),
    ] {
        assert_eq!(
            size(&[ftyp.clone(), meta(&inner), mdat.clone()].concat()),
            None
        );
    }
    let ipco = bx(b"iprp", &bx(b"ipco", &[0, 0, 0, 9]));
    assert_eq!(size(&[ftyp, meta(&ipco), mdat].concat()), None);
}

#[test]
fn a_box_may_run_to_the_end_or_carry_a_64_bit_size() {
    let mut f = file(b"avif", &ispe(7, 8));
    // Rewrite the trailing `mdat` (11 bytes) with size 0: "to the end".
    let at = f.len() - 11;
    f[at..at + 4].copy_from_slice(&[0; 4]);
    assert_eq!(size(&f), Some((7, 8)));
    // The same `mdat` as a 64-bit "largesize" box (size field 1).
    let mut g = f[..at].to_vec();
    g.extend_from_slice(&1u32.to_be_bytes());
    g.extend_from_slice(b"mdat");
    g.extend_from_slice(&19u64.to_be_bytes());
    g.extend_from_slice(&[1, 2, 3]);
    assert_eq!(size(&g), Some((7, 8)));
    // A largesize cut before its 8 bytes, and a size smaller than a header.
    assert_eq!(size(&g[..g.len() - 6]), None);
    let mut h = f[..at].to_vec();
    h.extend_from_slice(&4u32.to_be_bytes());
    h.extend_from_slice(b"mdat");
    assert_eq!(size(&h), None);
}
