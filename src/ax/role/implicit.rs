//! Implicit (HTML-AAM) role mapping, conditions included.
//!
//! One `match` on the tag name; an arm that the HTML-AAM makes conditional
//! calls a predicate over the document rather than getting its own branch in
//! the caller. The conditional arms, and the browser evidence behind each:
//!
//! - **`<input>`** — the full type table (§ [`input_role`]). `type=hidden` maps
//!   to *no role*: it is not rendered, so it is not exposed. Its layout twin
//!   lives in the cascade (`Element::hidden_input`), because a role table
//!   cannot tell `bboxes` anything.
//! - **`<section>`, `<form>`** — landmark only when the element has an author
//!   name; unnamed ones are `generic`. This is the single biggest source of
//!   landmark noise (USA.gov exposed 7 sections where Chrome exposes 3).
//! - **`<aside>`** — `complementary`, but inside sectioning content it needs a
//!   name too, else `generic`.
//! - **`<header>`/`<footer>`** — `banner`/`contentinfo` only when no sectioning
//!   ancestor stands between them and `<body>`; inside one they are
//!   `sectionheader`/`sectionfooter` (Stanford: 1 banner + 10 sectionheaders,
//!   1 contentinfo + 1 sectionfooter from a `<blockquote>`).
//! - **`<th>`** — `rowheader` or `columnheader` from `scope`, else from its
//!   position in the row (Wikipedia: 14 `scope=row` cells are rowheaders).
//! - **`<td>`/`<th>`** — `cell`/`gridcell` follows the ancestor table's role.
//! - **`<li>`** — `listitem` only inside `<ul>`/`<ol>`/`<menu>`.
//! - **`<img>`** — `presentation` for `alt=""`, but only when nothing else
//!   names it: a name is a conflict that keeps the node exposed.
//! - **`<a>`/`<area>`, `<select>`** — the pre-existing `href`/`multiple`/`size`
//!   conditions, unchanged.
//!
//! Tags mapped to a *fixed* role are the flat rows below; each was audited
//! against the HTML-AAM and carries no condition.

use super::context;
use crate::dom::{Document, Element, NodeId};

pub fn role(doc: &Document, id: NodeId, el: &Element) -> Option<&'static str> {
    let fixed = match el.name.as_str() {
        "a" | "area" => return Some(link_or_generic(el)),
        "aside" => return Some(context::scoped_aside(doc, id, el)),
        "footer" => {
            return Some(context::header_footer(
                doc,
                id,
                "contentinfo",
                "sectionfooter",
            ))
        }
        "header" => return Some(context::header_footer(doc, id, "banner", "sectionheader")),
        "form" => return Some(context::named_landmark(doc, el, "form")),
        "section" => return Some(context::named_landmark(doc, el, "region")),
        "img" => return Some(img_role(doc, el)),
        "input" => return input_role(el),
        "li" => return Some(context::listitem_or_generic(doc, id)),
        "select" => return Some(select_role(el)),
        "td" => return Some(context::cell_role(doc, id, "cell", "gridcell")),
        "th" => return Some(th_role(doc, id, el)),
        "article" => "article",
        "blockquote" => "blockquote",
        "body" | "div" | "span" => "generic",
        "button" | "summary" => "button",
        "caption" | "figcaption" => "caption",
        "code" => "code",
        "datalist" => "listbox",
        "dd" => "definition",
        "del" | "s" => "deletion",
        "details" | "fieldset" | "hgroup" | "optgroup" => "group",
        "dfn" | "dt" => "term",
        "dialog" => "dialog",
        "dl" | "menu" | "ol" | "ul" => "list",
        "em" => "emphasis",
        "figure" => "figure",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        "hr" => "separator",
        "ins" => "insertion",
        "main" => "main",
        "mark" => "mark",
        "math" => "math",
        "meter" => "meter",
        "nav" => "navigation",
        "option" => "option",
        "output" => "status",
        "p" => "paragraph",
        "progress" => "progressbar",
        "search" => "search",
        "strong" => "strong",
        "sub" => "subscript",
        "sup" => "superscript",
        "svg" => "graphics-document",
        "table" => "table",
        "tbody" | "tfoot" | "thead" => "rowgroup",
        "textarea" => "textbox",
        "time" => "time",
        "tr" => "row",
        _ => return None,
    };
    Some(fixed)
}

fn link_or_generic(el: &Element) -> &'static str {
    if el.attr("href").is_some() {
        "link"
    } else {
        "generic"
    }
}

/// `alt=""` declares the image decorative — unless something else names it, in
/// which case the author contradicted themselves and the name wins (browsers
/// keep the node rather than drop a named element).
fn img_role(doc: &Document, el: &Element) -> &'static str {
    if el.attr("alt") == Some("") && context::author_name(doc, el).is_none() {
        "presentation"
    } else {
        "img"
    }
}

/// The HTML-AAM `input` table, keyed on the `type` attribute (missing or
/// unrecognized ⇒ `text`, as the HTML parser itself treats it).
///
/// `hidden` is the only type with no role: it is never rendered. The types the
/// HTML-AAM leaves with "no corresponding role" but which *are* rendered
/// controls — `password`, `color`, `file`, and the date/time family — fall to
/// `textbox` rather than vanishing: a coarse role loses less than a dropped
/// node. A `list` attribute turns the text-entry types into a `combobox`,
/// which is what the datalist popup makes them.
fn input_role(el: &Element) -> Option<&'static str> {
    let t = el.attr("type").unwrap_or("text").to_ascii_lowercase();
    let listed = el.attr("list").is_some();
    Some(match t.as_str() {
        "hidden" => return None,
        "button" | "image" | "reset" | "submit" => "button",
        "checkbox" => "checkbox",
        "radio" => "radio",
        "range" => "slider",
        "number" => "spinbutton",
        "search" if listed => "combobox",
        "search" => "searchbox",
        "email" | "tel" | "text" | "url" if listed => "combobox",
        _ => "textbox",
    })
}

fn select_role(el: &Element) -> &'static str {
    let multiple = el.attr("multiple").is_some();
    let size: u32 = el.attr("size").and_then(|s| s.parse().ok()).unwrap_or(0);
    if multiple || size > 1 {
        "listbox"
    } else {
        "combobox"
    }
}

/// A `<th>` is a header for a column or for a row. `scope` says which when the
/// author supplied it; otherwise the position decides, per the same DOM-only
/// reading Chromium applies: a `<th>` that opens a row whose remaining cells
/// are data cells heads that *row*, anything else heads its column.
fn th_role(doc: &Document, id: NodeId, el: &Element) -> &'static str {
    let scope = el.attr("scope").unwrap_or("").to_ascii_lowercase();
    match scope.as_str() {
        "row" | "rowgroup" => "rowheader",
        "col" | "colgroup" => "columnheader",
        _ if context::heads_its_row(doc, id) => "rowheader",
        _ => "columnheader",
    }
}
