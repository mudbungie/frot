//! ARIA role resolution.
//!
//! [`role`] returns the effective role for an element: an explicit `role=`
//! attribute if it names a known role, otherwise the element's implicit role
//! per the HTML AAM. Returns `None` when no semantic role applies (e.g.
//! `<br>`, `<link>`, `<meta>`).
//!
//! [`level`] extracts the integer level for heading roles (h1–h6), honoring
//! `aria-level` if present.

use crate::dom::Element;

/// Effective ARIA role for an element, or `None` for elements that contribute
/// no semantics at all.
pub fn role(el: &Element) -> Option<&'static str> {
    if let Some(r) = el.attr("role") {
        let token = r.split_whitespace().next().unwrap_or("");
        if !token.is_empty() {
            if let Some(found) = KNOWN_ROLES.iter().find(|known| **known == token) {
                return Some(*found);
            }
        }
    }
    implicit_role(el)
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

fn implicit_role(el: &Element) -> Option<&'static str> {
    match el.name.as_str() {
        "a" | "area" => {
            if el.attr("href").is_some() {
                Some("link")
            } else {
                Some("generic")
            }
        }
        "article" => Some("article"),
        "aside" => Some("complementary"),
        "blockquote" => Some("blockquote"),
        "body" => Some("generic"),
        "button" => Some("button"),
        "caption" => Some("caption"),
        "code" => Some("code"),
        "datalist" => Some("listbox"),
        "dd" => Some("definition"),
        "del" | "s" => Some("deletion"),
        "details" => Some("group"),
        "dfn" => Some("term"),
        "dialog" => Some("dialog"),
        "div" | "span" => Some("generic"),
        "dl" => Some("list"),
        "dt" => Some("term"),
        "em" => Some("emphasis"),
        "fieldset" => Some("group"),
        "figcaption" => Some("caption"),
        "figure" => Some("figure"),
        "footer" => Some("contentinfo"),
        "form" => Some("form"),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some("heading"),
        "header" => Some("banner"),
        "hr" => Some("separator"),
        "img" => {
            if el.attr("alt").map(|s| s.is_empty()).unwrap_or(false) {
                Some("presentation")
            } else {
                Some("img")
            }
        }
        "input" => Some(input_role(el)),
        "ins" => Some("insertion"),
        "li" => Some("listitem"),
        "main" => Some("main"),
        "mark" => Some("mark"),
        "math" => Some("math"),
        "menu" => Some("list"),
        "meter" => Some("meter"),
        "nav" => Some("navigation"),
        "ol" | "ul" => Some("list"),
        "optgroup" => Some("group"),
        "option" => Some("option"),
        "output" => Some("status"),
        "p" => Some("paragraph"),
        "progress" => Some("progressbar"),
        "section" => Some("region"),
        "select" => Some(select_role(el)),
        "strong" => Some("strong"),
        "sub" => Some("subscript"),
        "summary" => Some("button"),
        "sup" => Some("superscript"),
        "svg" => Some("graphics-document"),
        "table" => Some("table"),
        "tbody" | "tfoot" | "thead" => Some("rowgroup"),
        "td" => Some("cell"),
        "textarea" => Some("textbox"),
        "th" => Some("columnheader"),
        "time" => Some("time"),
        "tr" => Some("row"),
        _ => None,
    }
}

fn input_role(el: &Element) -> &'static str {
    let raw = el.attr("type").unwrap_or("text").to_ascii_lowercase();
    match raw.as_str() {
        "button" | "image" | "reset" | "submit" => "button",
        "checkbox" => "checkbox",
        "radio" => "radio",
        "range" => "slider",
        "search" => "searchbox",
        "email" | "tel" | "url" | "password" | "number" | "text" => "textbox",
        _ => "textbox",
    }
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
