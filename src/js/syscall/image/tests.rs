//! The `<img>` sniff against real files (encoded by a standard library, the
//! same 37×23 image in each format) and against the truncations Firefox 153.0esr
//! fails — every format loads when whole and errors when cut short (measured).

use super::*;

const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAACUAAAAXCAIAAAADThHyAAAAKElEQVR4nGM8wcXFQEfARE/LRu0btW/UvlH7Ru0btW/UvlH7Ru0jCACWzQEKEBKtfgAAAABJRU5ErkJggg==";
const GIF: &str = "R0lGODdhJQAXAIEAAMgKCgAAAAAAAAAAACwAAAAAJQAXAEAIMQABCBxIsKDBgwgTKlzIsKHDhxAjSpxIsaLFixgzatzIsaPHjyBDihxJsqTJkyg/BgQAOw==";
const VP8: &str = "UklGRkwAAABXRUJQVlA4IEAAAABQAwCdASolABcAPm02l0ikIyIhJWgAgA2JZwB2AABYzxq0AAD+8iJf/+xZy2BKD//7nA/7nA/7nA/isc2A0AAA";
const VP8L: &str = "UklGRh4AAABXRUJQVlA4TBEAAAAvJIAFAAdQhSJXof+BiOh/AAA=";
const VP8X: &str = "UklGRnAAAABXRUJQVlA4WAoAAAAQAAAAJAAAFgAAQUxQSAoAAAABB1DAiAhERP8DVlA4IEAAAABQAwCdASolABcAPm02l0ikIyIhJWgAgA2JZwB2AABYzxq0AAD+8iJf/+xZy2BKD//7nA/7nA/7nA/isc2A0AAA";
const ICO: &str = "AAABAAEAEAoAAAAAIABQAAAAFgAAAIlQTkcNChoKAAAADUlIRFIAAAAQAAAACggCAAAAMtxJywAAABdJREFUeJxjPMHFxUAKYCJJ9aiGQaMBALk7APBC6zjJAAAAAElFTkSuQmCC";
const JPEG: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAAXACUDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwDgqKKK8M/VAooooAKKKKACiiigAooooAKKKKAP/9k=";

/// The bytes a base64 fixture carries, through the one shared decoder.
fn raw(b64: &str) -> Vec<u8> {
    data::bytes(&format!("image/x;base64,{b64}")).unwrap().1
}

/// `bytes` as a percent-encoded `data:` URL of type `mime`.
fn url(mime: &str, bytes: &[u8]) -> String {
    let body: String = bytes.iter().map(|b| format!("%{b:02X}")).collect();
    format!("data:{mime},{body}")
}

fn sized(bytes: &[u8]) -> Option<(u32, u32)> {
    size(&url("image/x", bytes))
}

/// Every prefix of a whole file shorter than it fails, as a header-only file
/// does in Firefox — the walk never reads past what is there.
fn every_prefix_fails(bytes: &[u8]) {
    for n in 0..bytes.len() {
        assert_eq!(raster(&bytes[..n]), None, "{n} of {}", bytes.len());
    }
}

#[test]
fn real_files_size_and_their_truncations_fail() {
    for b64 in [PNG, GIF, VP8, VP8L, VP8X, JPEG] {
        let bytes = raw(b64);
        assert_eq!(sized(&bytes), Some((37, 23)), "{b64}");
        every_prefix_fails(&bytes);
    }
    // ICO's first entry is 16x10 (a PNG-in-ICO the encoder resized).
    assert_eq!(sized(&raw(ICO)), Some((16, 10)));
    every_prefix_fails(&raw(ICO));
    // Sniffed, whatever the declared type (measured: a PNG as text/plain loads).
    assert_eq!(
        size(&format!("data:text/plain;base64,{PNG}")),
        Some((37, 23))
    );
}

#[test]
fn what_is_not_an_image_frot_has_is_none() {
    assert_eq!(size("https://example.com/a.png"), None);
    assert_eq!(size("data:image/png;base64,@@@"), None);
    assert_eq!(size("data:,hello"), None);
    assert_eq!(size("data:image/svg+xml;base64,/w=="), None); // not UTF-8
}

#[test]
fn a_png_needs_ihdr_first() {
    let mut b = raw(PNG);
    b[12] = b'X';
    assert_eq!(sized(&b), None);
}

#[test]
fn a_jpeg_walk_stops_at_a_non_marker() {
    let mut b = raw(JPEG);
    b[2] = 0;
    assert_eq!(sized(&b), None);
}

#[test]
fn a_webp_needs_a_known_first_chunk() {
    let mut b = raw(VP8);
    b[15] = b'Q';
    assert_eq!(sized(&b), None);
}

/// Hand-made BMPs: the encoder's are 3.5 KB of pixels, and the header is the
/// whole point. A negative height is a top-down bitmap of the same size.
#[test]
fn a_bmp_is_sized_by_its_info_header() {
    let bmp = |h: i32| {
        let mut b = b"BM".to_vec();
        b.extend(30u32.to_le_bytes());
        b.extend([0; 12]);
        b.extend(37i32.to_le_bytes());
        b.extend(h.to_le_bytes());
        b.extend([0; 4]);
        b
    };
    assert_eq!(sized(&bmp(23)), Some((37, 23)));
    assert_eq!(sized(&bmp(-23)), Some((37, 23)));
    every_prefix_fails(&bmp(23));
}

#[test]
fn an_ico_side_of_zero_is_256() {
    let mut b = raw(ICO);
    b[6] = 0;
    b[7] = 0;
    assert_eq!(sized(&b), Some((256, 256)));
}

fn svg(doc: &str) -> Option<(u32, u32)> {
    size(&format!("data:image/svg+xml;charset=utf-8,{doc}"))
}

/// The sizes Firefox 153.0esr reported for the same documents.
#[test]
fn an_svg_is_sized_like_firefox_sizes_it() {
    let ns = "xmlns=\"http://www.w3.org/2000/svg\"";
    for (attrs, want) in [
        ("width=\"40\" height=\"30\"", (40, 30)),
        ("width=\"40px\" height='30.6'", (40, 31)),
        ("viewBox=\"0 0 80 60\"", (300, 225)),
        ("width=\"40\" viewBox=\"0,0,80,60\"", (40, 30)),
        ("height=\"30\" viewBox=\"0 0 80 60\"", (40, 30)),
        ("", (300, 150)),
        ("width=\"50%\" height=\"20em\"", (300, 150)),
        ("width=\"-4\" height=\"x\"", (300, 150)),
        ("width=\"40\"", (40, 150)),
        ("viewBox=\"0 0 80\"", (300, 150)),
        ("viewBox=\"0 0 0 60\"", (300, 150)),
        ("viewBox=\"0 0 a 60\"", (300, 150)),
        // A name inside another name, or not followed by `=`, is not it.
        (
            "stroke-width=\"9\" xmlns:xlink=\"x\" width=\"40\"",
            (40, 150),
        ),
    ] {
        assert_eq!(svg(&format!("<svg {ns} {attrs}/>")), Some(want), "{attrs}");
        assert_eq!(
            svg(&format!("<svg {attrs} {ns}></svg>")),
            Some(want),
            "{attrs}"
        );
    }
    let prolog = "<?xml version=\"1.0\"?>\n<!-- c --><!DOCTYPE svg>";
    assert_eq!(
        svg(&format!("{prolog}<svg {ns} width='5' height='6'/>")),
        Some((5, 6))
    );
}

/// Firefox fails each of these (measured: no namespace, an unclosed root,
/// non-SVG markup, an SVG declared as text).
#[test]
fn an_svg_that_is_not_one_fails() {
    let ns = "xmlns=\"http://www.w3.org/2000/svg\"";
    for doc in [
        "<svg width=\"40\" height=\"30\"/>".to_string(),
        format!("<svg {ns} width=\"40\">"),
        format!("<svg {ns}/><p>"),
        format!("<svgx {ns}/>"),
        format!("<svg {ns}"),
        format!("<svg xmlns={ns}/>"),
        "<svg xmlns=\"http://www.w3.org/2000/svg/>".to_string(),
        "<p/>".to_string(),
        "<!-- unterminated".to_string(),
    ] {
        assert_eq!(svg(&doc), None, "{doc}");
    }
    assert_eq!(size(&format!("data:text/plain,<svg {ns}/>")), None);
    assert_eq!(svg("<svg/>"), None);
}
