//! Which image an `<img>` requests (`bl-0e4a`, js.md §7 "Images"): the HTML
//! source selection, with the one choice the spec leaves to the browser made
//! as Firefox 153.0esr makes it, at the persona's device pixel ratio of 1.
//!
//! In the order it happens:
//! 1. *The source set.* In a `<picture>`, each `<source>` before the `<img>`
//!    is tried in order: it is taken when its `srcset` yields a candidate, its
//!    `media` matches and its `type` is one Firefox decodes. Otherwise the
//!    `<img>`'s own `srcset`, plus its `src` as a `1x` candidate when the set
//!    has neither a `1x` nor a `w` candidate.
//! 2. *Densities.* An `x` candidate's is its own; a `w` candidate's is its
//!    width over the source size (`sizes`, its `auto` only on a lazy image).
//! 3. *The pick* (Firefox's `ResponsiveImageSelector`): the lowest density
//!    not below 1, else the highest; the first on a tie.

use super::srcset::{self, Cand, Desc};
use crate::css::media;
use crate::dom::{Document, NodeId};

/// The image types a `<source type>` may name (measured, essence only and
/// case-insensitive; `image/jxl`, `image/tiff`, `image/heic`, `image/jp2` and
/// `image/avif-sequence` are refused).
const TYPES: &[&str] = &[
    "image/png",
    "image/x-png",
    "image/apng",
    "image/jpeg",
    "image/jpg",
    "image/pjpeg",
    "image/gif",
    "image/webp",
    "image/avif",
    "image/bmp",
    "image/x-ms-bmp",
    "image/svg+xml",
    "image/x-icon",
    "image/vnd.microsoft.icon",
    "image/icon",
];

/// The concrete width `sizes=auto` stands for on a lazy image without a
/// `width` attribute — the default object size (measured: 300).
const AUTO_DEFAULT: f64 = 300.0;

/// The URL `img` requests, as written, and the density it is shown at — or
/// `None` when no source offers a candidate.
pub fn select(doc: &Document, img: NodeId) -> Option<(String, f64)> {
    let el = doc.element(img)?;
    let auto = el
        .attr("loading")
        .filter(|l| l.eq_ignore_ascii_case("lazy"))
        .map(|_| {
            el.attr("width")
                .and_then(|w| w.parse::<u32>().ok())
                .map_or(AUTO_DEFAULT, f64::from)
        });
    let picture = doc
        .node(img)
        .parent
        .filter(|&p| doc.element(p).is_some_and(|e| e.name == "picture"));
    let sources = picture.map_or(&[][..], |p| &doc.node(p).children[..]);
    for &s in sources.iter().take_while(|&&s| s != img) {
        let Some(src) = doc.element(s).filter(|e| e.name == "source") else {
            continue;
        };
        let cands = srcset::parse(src.attr("srcset").unwrap_or_default());
        let fits =
            src.attr("media").is_none_or(media::matches) && src.attr("type").is_none_or(decodable);
        if !cands.is_empty() && fits {
            return pick(cands, srcset::size(src.attr("sizes"), auto));
        }
    }
    let mut cands = srcset::parse(el.attr("srcset").unwrap_or_default());
    let default = !cands
        .iter()
        .any(|c| matches!(c.desc, Desc::W(_)) || c.desc == Desc::X(1.0));
    if let Some(src) = el.attr("src").filter(|s| default && !s.is_empty()) {
        cands.push(Cand {
            url: src.to_string(),
            desc: Desc::X(1.0),
        });
    }
    pick(cands, srcset::size(el.attr("sizes"), auto))
}

/// Whether a `<source type>` names an image Firefox decodes; an empty type
/// names none and so restricts nothing.
fn decodable(t: &str) -> bool {
    let essence = t.split(';').next().unwrap_or_default().trim();
    essence.is_empty() || TYPES.iter().any(|k| k.eq_ignore_ascii_case(essence))
}

/// Firefox's pick over `cands` at device pixel ratio 1, `size` the source size.
fn pick(cands: Vec<Cand>, size: f64) -> Option<(String, f64)> {
    let mut best: Option<(String, f64)> = None;
    for c in cands {
        let d = match c.desc {
            Desc::X(d) => d,
            Desc::W(w) => f64::from(w) / size,
        };
        let take = best
            .as_ref()
            .is_none_or(|&(_, b)| (b < 1.0 && d > b) || (d >= 1.0 && d < b));
        if take {
            best = Some((c.url, d));
        }
    }
    best
}

#[cfg(test)]
mod tests;
