//! Whether a list-item box has a marker: its computed `list-style-type` is
//! `none` or not (`bl-08c6`). Measured in Chrome 158 (`getFullAXTree`, the
//! `ListMarker` under a `<p style="display:list-item">`):
//!
//! - `list-style-type` and the `list-style` shorthand compete as one
//!   property: the cascade's winner of the two decides (`list-style-type:
//!   none; list-style: square` has a marker, the reverse order has none).
//! - In the shorthand, `none` is the type only when no other word is one:
//!   `list-style: none`, `inside none` and `none none` have no marker,
//!   `square none` and `none square` have one (the `none` is the image's).
//! - It is inherited (`<div style="list-style:none">` takes the marker off a
//!   list item inside), `inherit` and `unset` take the parent's, `initial`
//!   is `disc`, and any other name — `bogus` too, which counts in decimal —
//!   is a marker. Letter case does not matter.
//!
//! An image (`url(…)`) is never a marker here: frot loads no images, and a
//! list item whose image did not load shows its type.

use super::computed::Applied;

/// Whether the winning `list-style-type` is `none`; `parent` is the
/// inherited answer.
pub fn unmarked(applied: &[Applied], parent: bool) -> bool {
    let winner = applied
        .iter()
        .filter(|a| {
            a.pseudo.is_none() && matches!(a.name.as_str(), "list-style-type" | "list-style")
        })
        .max_by_key(|a| a.key());
    let Some(a) = winner else {
        return parent;
    };
    let value = a.value.to_ascii_lowercase();
    let words: Vec<&str> = value.split_ascii_whitespace().collect();
    let not_type = |w: &&str| matches!(*w, "none" | "inside" | "outside") || w.starts_with("url(");
    match words.as_slice() {
        ["inherit" | "unset"] => parent,
        _ if a.name == "list-style-type" => words == ["none"],
        _ => words.contains(&"none") && words.iter().all(not_type),
    }
}

#[cfg(test)]
mod tests {
    use crate::css::compute;
    use crate::dom::Document;

    /// Whether the last `<p>` of `html` has a marker.
    fn marked(html: &str) -> bool {
        let doc = Document::parse(html);
        let p = *doc.find_by_tag("p").last().unwrap();
        compute(&doc).marked(p)
    }

    #[test]
    fn list_style_type_none_takes_the_marker_off() {
        let sheet = |css: &str, body: &str| format!("<style>{css}</style>{body}");
        for (html, want) in [
            ("<p></p>".to_string(), true),
            ("<p style='list-style-type:none'></p>".into(), false),
            ("<p style='list-style-type:NONE'></p>".into(), false),
            ("<p style='list-style-type:bogus'></p>".into(), true),
            ("<p style='list-style:none'></p>".into(), false),
            ("<p style='list-style:inside none'></p>".into(), false),
            ("<p style='list-style:none none'></p>".into(), false),
            ("<p style='list-style:url(x.png) none'></p>".into(), false),
            ("<p style='list-style:square none'></p>".into(), true),
            ("<p style='list-style:inside'></p>".into(), true),
            ("<div style='list-style:none'><p></p></div>".into(), false),
            (
                "<div style='list-style:none'><p style='list-style-type:inherit'></p></div>".into(),
                false,
            ),
            (
                "<div style='list-style:none'><p style='list-style-type:unset'></p></div>".into(),
                false,
            ),
            (
                "<div style='list-style:none'><p style='list-style-type:initial'></p></div>".into(),
                true,
            ),
            (
                sheet("p{list-style-type:none;list-style:square}", "<p></p>"),
                true,
            ),
            (
                sheet("p{list-style:square;list-style-type:none}", "<p></p>"),
                false,
            ),
            (sheet("p::before{list-style:none}", "<p></p>"), true),
        ] {
            assert_eq!(marked(&html), want, "{html}");
        }
    }
}
