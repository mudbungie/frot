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
//! - **`<section>`** — a `region` only when the element has an author name;
//!   unnamed it is `generic`. This is the single biggest source of landmark
//!   noise (USA.gov exposed 7 sections where Chrome exposes 3). `<form>` is
//!   *not* conditional: Chrome 156 exposes an unnamed `<form>` as `form ""`
//!   wherever it stands — top level, inside `<search>`, empty — so it is a flat
//!   row below (`bl-2a3c`). Whether a nameless form counts as a *landmark* is a
//!   question for whoever navigates the tree, not for the tree.
//! - **`<tbody>`** — a `rowgroup` only when the author named it, else `generic`
//!   and its rows sit directly under the table. `<thead>` and `<tfoot>` are
//!   always rowgroups. Measured, Chrome 156 (`bl-e760`): the HTML-AAM maps all
//!   three to `rowgroup`, but Chrome leaves an unnamed `<tbody>` — explicit or
//!   parser-implied — out of the tree as uninteresting.
//! - **`<aside>`** — `complementary`, but inside sectioning content it needs a
//!   name too, else `generic`.
//! - **`<header>`/`<footer>`** — `banner`/`contentinfo` only when no sectioning
//!   ancestor stands between them and `<body>`; inside one they are
//!   `sectionheader`/`sectionfooter` (Stanford: 1 banner + 10 sectionheaders,
//!   1 contentinfo + 1 sectionfooter from a `<blockquote>`).
//! - **`<th>`** — `rowheader` or `columnheader` from `scope`, else from its
//!   position in the row (Wikipedia: 14 `scope=row` cells are rowheaders).
//! - **`<td>`/`<th>`** — `cell`/`gridcell` follows the ancestor table's role.
//! - **`<li>`** — `listitem` only inside a `<ul>`/`<ol>`/`<menu>` that is still
//!   a list; **`<thead>`/`<tbody>`/`<tfoot>`/`<tr>`/`<td>`/`<th>`** — their
//!   roles only inside a `<table>` that is still a table or grid. An author
//!   `role=presentation` (or any other role) on the owner takes the parts'
//!   roles with it ([`context::table_part`], `bl-2ea9`).
//! - **`<img>`** — `presentation` for `alt=""`, but only when nothing else
//!   names it: a name is a conflict that keeps the node exposed.
//! - **`<a>`/`<area>`** — the pre-existing `href` condition, unchanged.
//! - **`<select>`** — `listbox` or `combobox` by its mode (`dom/select.rs`).
//! - **`<summary>`** — a `button` only as a child of `<details>`
//!   ([`context::summary`]).
//!
//! - **`<svg>` and its parts** — `super::svg`.
//! - **`<caption>`** — always a `caption` here; the one with no line in it is
//!   left out by the tree, which has the cascade (`ax/tree/lineless.rs`).
//!
//! Tags mapped to a *fixed* role are the flat rows below; each was audited
//! against the HTML-AAM and carries no condition.

use super::{context, svg};
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
        "section" => return Some(context::named_landmark(doc, el, "region")),
        "img" => return Some(img_role(doc, el)),
        "input" => return input_role(el),
        "li" => return Some(context::listitem_or_generic(doc, id)),
        "select" => return Some(select_role(el)),
        "svg" => return svg::svg(doc, id, el),
        "tbody" | "td" | "tfoot" | "th" | "thead" | "tr" => {
            return Some(context::table_part(doc, id, table_part(doc, id, el)))
        }
        "article" => "article",
        "blockquote" => "blockquote",
        "body" | "div" | "span" => "generic",
        "button" => "button",
        "summary" => return Some(context::summary(doc, id)),
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
        "form" => "form",
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
        "table" => "table",
        "textarea" => "textbox",
        "time" => "time",
        _ => return svg::part(doc, id, el),
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

/// The role a part of a `<table>` has while its table is one
/// ([`context::table_part`]).
fn table_part(doc: &Document, id: NodeId, el: &Element) -> &'static str {
    match el.name.as_str() {
        "tbody" => context::named_rowgroup(doc, el),
        "td" => context::cell_role(doc, id, "cell", "gridcell"),
        "th" => th_role(doc, id, el),
        "tr" => "row",
        _ => "rowgroup",
    }
}

/// `listbox` or `combobox` by the select's mode — [`Element::list_box`], the
/// one home of that table, which layout reads too.
fn select_role(el: &Element) -> &'static str {
    if el.list_box() {
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
