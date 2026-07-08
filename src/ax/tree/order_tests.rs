//! Reading-order refinement (subtask 3.7): under a computed `Layout`, a flex
//! container emits its children in [`Layout::child_order`] (visual reading
//! order) rather than source order; every non-flex container — and every
//! container when no layout is present — is unchanged. These tests drive the
//! `ax_tree` layout path; the source-order path (no layout) is the sibling
//! `tests` module, unedited.

use super::*;

/// Build the `ax` tree for `html` with a computed `Layout` — the `--css` path.
/// Inline `style=` carries `display`/`order`/`flex-direction` (as the layout and
/// flex unit tests do), so a fixture needs no external sheet.
fn ax_laid_out(html: &str) -> Value {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    let layout = crate::layout::compute(&doc, &styles, crate::layout::VIEWPORT_WIDTH);
    ax_tree(&doc, Some(&styles), Some(&layout))
}

/// Names of every node with `role`, in emission (array/DFS) order.
fn names(v: &Value, role: &str) -> Vec<String> {
    let mut out = Vec::new();
    collect(v, role, &mut out);
    out
}

fn collect(v: &Value, role: &str, out: &mut Vec<String>) {
    if let Value::Array(arr) = v {
        for item in arr {
            if item["role"] == role {
                out.push(item["name"].as_str().unwrap_or("").to_string());
            }
            collect(&item["children"], role, out);
        }
    }
}

#[test]
fn flex_order_property_emits_children_in_reading_order() {
    // order: A=2, B=1, C=0(default) → reading order [C, B, A]. The div is
    // generic and collapses, promoting its (reordered) link children.
    let v = ax_laid_out(
        "<div style=\"display:flex\">\
         <a href=\"/1\" style=\"order:2\">A</a>\
         <a href=\"/2\" style=\"order:1\">B</a>\
         <a href=\"/3\">C</a></div>",
    );
    assert_eq!(names(&v, "link"), vec!["C", "B", "A"]);
}

#[test]
fn flex_row_reverse_reverses_children() {
    let v = ax_laid_out(
        "<div style=\"display:flex;flex-direction:row-reverse\">\
         <a href=\"/1\">A</a><a href=\"/2\">B</a></div>",
    );
    assert_eq!(names(&v, "link"), vec!["B", "A"]);
}

#[test]
fn inline_flex_also_reorders() {
    // The refinement fires for `inline-flex` too — `child_order` is stored for
    // both flex kinds.
    let v = ax_laid_out(
        "<span style=\"display:inline-flex\">\
         <a href=\"/1\" style=\"order:1\">A</a>\
         <a href=\"/2\">B</a></span>",
    );
    assert_eq!(names(&v, "link"), vec!["B", "A"]);
}

#[test]
fn non_flex_container_keeps_source_order_under_layout() {
    // Same `order:` attributes, but a block container: layout is present yet the
    // guard sees a non-flex display, so children stay in source order — and none
    // are dropped (a stray `child_order` call would return the empty vec).
    let v = ax_laid_out(
        "<div style=\"display:block\">\
         <a href=\"/1\" style=\"order:2\">A</a>\
         <a href=\"/2\" style=\"order:1\">B</a></div>",
    );
    assert_eq!(names(&v, "link"), vec!["A", "B"]);
}
