//! Which elements can hold a `::before` / `::after` box (`bl-082b`).
//!
//! Generated content is a child box of the element's own, so an element
//! whose box takes no children generates none, whatever `content` says: the
//! box the stylesheet asks for is never built. Measured in Chrome 158 by
//! asking layout for the pseudo-element's box (`DOM.getBoxModel` on the
//! `::before` node of `.g::before { content: "BB" }`). No box for:
//!
//! - a replaced element or a line break — `<img>`, `<video>`, `<audio>`,
//!   `<canvas>`, `<iframe>`, `<embed>`, `<br>`, `<wbr>` — and a `<col>` or
//!   `<colgroup>`;
//! - a text control: `<textarea>`, and an `<input>` of every type but
//!   `checkbox`, `radio`, `range`, `color`, `file` and the date and time
//!   family — the text-entry and the button types (`text`, a missing or
//!   unknown type, `search`, `email`, `number`, `submit`, `image`, …) have
//!   none, while `input[type=checkbox]::before` is painted;
//! - a menu-list `<select>` and everything in it (a list box and its options
//!   do have them);
//! - `<svg>` and what is inside it, down to a `<foreignObject>`, whose HTML
//!   content is ordinary again.
//!
//! An `<img>` that failed to load is not replaced and Chrome gives it the
//! boxes; frot does not load images and reads every `<img>` as one that did.
//! `<object>` keeps them, as it keeps its fallback (`tags::renders_children`).

use super::{Document, Element, NodeId};

/// The elements whose own box takes no children.
const CHILDLESS: &[&str] = &[
    "audio", "br", "canvas", "col", "colgroup", "embed", "iframe", "img", "textarea", "video",
    "wbr",
];

/// The `<input>` types that are not a text control or a button.
const INPUTS_WITH_BOXES: &[&str] = &[
    "checkbox",
    "color",
    "date",
    "datetime-local",
    "file",
    "month",
    "radio",
    "range",
    "time",
    "week",
];

impl Document {
    /// Whether the element `id` can hold `::before` / `::after` boxes
    /// (module docs).
    pub fn holds_generated(&self, id: NodeId, el: &Element) -> bool {
        let kind = el.attr("type").unwrap_or("").to_ascii_lowercase();
        if CHILDLESS.contains(&el.name.as_str())
            || (el.name == "input" && !INPUTS_WITH_BOXES.contains(&kind.as_str()))
        {
            return false;
        }
        // The nearest of these, the element itself included, decides.
        let mut at = Some(id);
        while let Some(n) = at {
            match self.element(n).map(|e| (e.name.as_str(), e)) {
                Some(("select", select)) => return select.list_box(),
                Some(("svg", _)) => return false,
                Some(("foreignobject", _)) => return true,
                _ => at = self.node(n).parent,
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::css::compute;
    use crate::dom::Document;

    /// Whether `.g::before` generates content on the `.g` element of `html`.
    fn generates(html: &str) -> bool {
        let page =
            format!("<style>.g::before{{content:'B'}}.g::after{{content:'A'}}</style>{html}");
        let doc = Document::parse(&page);
        let styles = compute(&doc);
        let g = (0..doc.len() as u32)
            .find(|&n| doc.element(n).is_some_and(|e| e.attr("class") == Some("g")))
            .expect("a .g element");
        assert_eq!(styles.before(g).is_some(), styles.after(g).is_some());
        styles.before(g).is_some()
    }

    #[test]
    fn a_box_that_takes_no_children_generates_nothing() {
        for html in [
            "<img class=g src=x.png alt=a>",
            "<br class=g>",
            "<wbr class=g>",
            "<video class=g></video>",
            "<audio class=g controls></audio>",
            "<canvas class=g></canvas>",
            "<iframe class=g></iframe>",
            "<embed class=g>",
            "<textarea class=g></textarea>",
            "<table><colgroup class=g></colgroup></table>",
            "<table><colgroup><col class=g></colgroup></table>",
        ] {
            assert!(!generates(html), "{html}");
        }
        for html in [
            "<span class=g>s</span>",
            "<button class=g>b</button>",
            "<hr class=g>",
            "<meter class=g></meter>",
            "<progress class=g></progress>",
            "<object class=g>fallback</object>",
            "<details class=g><summary>s</summary></details>",
        ] {
            assert!(generates(html), "{html}");
        }
    }

    #[test]
    fn an_input_generates_only_where_it_is_no_text_control_or_button() {
        for kind in [
            "text", "search", "password", "email", "tel", "url", "number", "button", "submit",
            "reset", "image", "bogus", "TEXT",
        ] {
            assert!(
                !generates(&format!("<input class=g type={kind}>")),
                "{kind}"
            );
        }
        assert!(!generates("<input class=g>"));
        for kind in [
            "checkbox",
            "radio",
            "range",
            "color",
            "file",
            "date",
            "time",
            "month",
            "week",
            "datetime-local",
            "CHECKBOX",
        ] {
            assert!(generates(&format!("<input class=g type={kind}>")), "{kind}");
        }
    }

    #[test]
    fn a_menu_list_select_generates_nothing_and_a_list_box_does() {
        for html in [
            "<select class=g><option>o</select>",
            "<select><option class=g>o</select>",
            "<select><optgroup class=g label=l><option>o</optgroup></select>",
            "<select><optgroup label=l><option class=g>o</optgroup></select>",
            "<select multiple size=1><option class=g>o</select>",
        ] {
            assert!(!generates(html), "{html}");
        }
        for html in [
            "<select class=g size=3><option>o</select>",
            "<select size=3><option class=g>o</select>",
            "<select multiple><optgroup label=l><option class=g>o</optgroup></select>",
            "<div><option class=g>o</option></div>",
        ] {
            assert!(generates(html), "{html}");
        }
    }

    #[test]
    fn svg_generates_nothing_down_to_a_foreign_object() {
        for html in [
            "<svg class=g></svg>",
            "<svg><text class=g>t</text></svg>",
            "<svg><a href=x><text><tspan class=g>t</tspan></text></a></svg>",
            "<svg><foreignObject><svg><g class=g></g></svg></foreignObject></svg>",
        ] {
            assert!(!generates(html), "{html}");
        }
        for html in [
            "<svg><foreignObject class=g></foreignObject></svg>",
            "<svg><foreignObject><div><b class=g>b</b></div></foreignObject></svg>",
        ] {
            assert!(generates(html), "{html}");
        }
    }
}
