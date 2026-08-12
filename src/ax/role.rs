//! ARIA role resolution.
//!
//! [`role`] returns the effective role of a node: an explicit `role=`
//! attribute if it names a known role, otherwise the element's implicit role
//! per the HTML-AAM. `None` means "no semantic role at all" — the node
//! contributes nothing to the AX tree (`<br>`, `<link>`, `<meta>`,
//! `<input type=hidden>`).
//!
//! ## Why the whole document, not just the element
//!
//! Most HTML-AAM mappings are *conditional*: `<th>` is a `rowheader` or a
//! `columnheader` depending on `scope` and its position in the row, `<footer>`
//! is `contentinfo` only outside a sectioning ancestor, `<section>` is a
//! `region` only when it is named, `<li>` is a `listitem` only inside a list.
//! A flat tag→role table cannot express any of that, so resolution takes
//! `(doc, id)` and reads the context it needs through [`context`]. There is one
//! resolver and it is the authority; nothing downstream re-derives a role.
//!
//! [`level`] extracts the integer level for heading roles (h1–h6), honoring
//! `aria-level` if present.

use crate::dom::{Document, Element, NodeId, NodeKind};

mod context;
mod implicit;

/// Effective ARIA role for the node `id`, or `None` when it carries no
/// semantics (a non-element node, or an element the HTML-AAM maps to nothing).
pub fn role(doc: &Document, id: NodeId) -> Option<&'static str> {
    let NodeKind::Element(el) = &doc.node(id).kind else {
        return None;
    };
    explicit(el).or_else(|| implicit::role(doc, id, el))
}

/// An author `role=` attribute, resolved to its first known token. An
/// unrecognized token is ignored entirely (the implicit role stands), which is
/// what browsers do with a typo'd role.
fn explicit(el: &Element) -> Option<&'static str> {
    let token = el.attr("role")?.split_whitespace().next()?;
    KNOWN_ROLES.iter().find(|known| **known == token).copied()
}

/// Heading level. Returns the `aria-level` value first if present and
/// parseable, otherwise 1..=6 for h1..h6, otherwise `None`.
pub fn level(el: &Element) -> Option<u32> {
    if let Some(s) = el.attr("aria-level") {
        if let Ok(n) = s.parse::<u32>() {
            if n >= 1 {
                return Some(n);
            }
        }
    }
    match el.name.as_str() {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

const KNOWN_ROLES: &[&str] = &[
    "alert",
    "alertdialog",
    "application",
    "article",
    "banner",
    "blockquote",
    "button",
    "caption",
    "cell",
    "checkbox",
    "code",
    "columnheader",
    "combobox",
    "complementary",
    "contentinfo",
    "definition",
    "deletion",
    "dialog",
    "directory",
    "document",
    "emphasis",
    "feed",
    "figure",
    "form",
    "generic",
    "graphics-document",
    "grid",
    "gridcell",
    "group",
    "heading",
    "img",
    "insertion",
    "link",
    "list",
    "listbox",
    "listitem",
    "log",
    "main",
    "mark",
    "marquee",
    "math",
    "menu",
    "menubar",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "navigation",
    "none",
    "note",
    "option",
    "paragraph",
    "presentation",
    "progressbar",
    "radio",
    "radiogroup",
    "region",
    "row",
    "rowgroup",
    "rowheader",
    "scrollbar",
    "search",
    "searchbox",
    "sectionfooter",
    "sectionheader",
    "separator",
    "slider",
    "spinbutton",
    "status",
    "strong",
    "subscript",
    "superscript",
    "switch",
    "tab",
    "table",
    "tablist",
    "tabpanel",
    "term",
    "textbox",
    "time",
    "timer",
    "toolbar",
    "tooltip",
    "tree",
    "treegrid",
    "treeitem",
];

#[cfg(test)]
mod tests;
