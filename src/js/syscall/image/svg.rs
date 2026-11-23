//! An SVG document's intrinsic size, read off its root `<svg>` start tag — the
//! trivial case, not an XML parser.
//!
//! Measured on Firefox 153.0esr: the root must carry the SVG namespace (without
//! it the image fails), and must be closed. Its size is `width`/`height` when
//! they are absolute lengths, rounded; a missing one (or a relative one, `%`)
//! is derived from the other through the `viewBox` aspect ratio, and failing
//! that defaults to the replaced-element 300 × 150 — so a bare `viewBox` is 300
//! wide and as tall as its ratio makes it. frot reads unitless and `px`
//! lengths; any other unit is treated as missing (the residual).

const NS: &str = "http://www.w3.org/2000/svg";

/// `(width, height)` of a whole SVG document with an SVG root, else `None`.
pub fn size(doc: &str) -> Option<(u32, u32)> {
    let rest = prolog(doc).strip_prefix("<svg")?;
    let end = rest.find('>')?;
    let (tag, after) = (&rest[..end], rest[end + 1..].trim());
    let closed = match tag.strip_suffix('/') {
        Some(_) => after.is_empty(),
        None => after.ends_with("</svg>"),
    };
    let tag = tag.strip_suffix('/').unwrap_or(tag);
    let named = tag.is_empty() || tag.starts_with(|c: char| c.is_ascii_whitespace());
    let attr = |name: &str| attr(tag, name);
    if !(named && closed && attr("xmlns") == Some(NS)) {
        return None;
    }
    let (w, h) = (
        attr("width").and_then(length),
        attr("height").and_then(length),
    );
    let ratio = attr("viewBox").and_then(view_box);
    let (w, h) = match (w, h, ratio) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, Some(r)) => (w, w / r),
        (None, Some(h), Some(r)) => (h * r, h),
        (None, None, Some(r)) => (300.0, 300.0 / r),
        (w, h, None) => (w.unwrap_or(300.0), h.unwrap_or(150.0)),
    };
    Some((w.round() as u32, h.round() as u32))
}

/// Skip what may precede the root: whitespace, the XML declaration and other
/// processing instructions, comments, a doctype.
fn prolog(mut s: &str) -> &str {
    loop {
        s = s.trim_start();
        let close = match () {
            _ if s.starts_with("<?") => "?>",
            _ if s.starts_with("<!--") => "-->",
            _ if s.starts_with("<!") => ">",
            _ => return s,
        };
        match s.find(close) {
            Some(i) => s = &s[i + close.len()..],
            None => return "",
        }
    }
}

/// The value of attribute `name` in a start tag's attribute text, quoted either
/// way.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    loop {
        let at = rest.find(name)?;
        let before = rest[..at].chars().next_back();
        let after = rest[at + name.len()..].trim_start();
        rest = &rest[at + name.len()..];
        let Some(value) = after.strip_prefix('=').map(str::trim_start) else {
            continue;
        };
        if !before.is_some_and(|c| c.is_ascii_whitespace()) {
            continue;
        }
        let quote = value.chars().next().filter(|q| *q == '"' || *q == '\'')?;
        let value = &value[1..];
        return value.find(quote).map(|end| &value[..end]);
    }
}

/// An absolute length in CSS pixels: a non-negative number, unitless or `px`.
fn length(v: &str) -> Option<f64> {
    let v = v.trim();
    let n: f64 = v.strip_suffix("px").unwrap_or(v).parse().ok()?;
    (n.is_finite() && n >= 0.0).then_some(n)
}

/// A `viewBox`'s width / height, when both are positive.
fn view_box(v: &str) -> Option<f64> {
    let n: Vec<f64> = v
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|p| !p.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    match n[..] {
        [_, _, w, h] if w > 0.0 && h > 0.0 => Some(w / h),
        _ => None,
    }
}
