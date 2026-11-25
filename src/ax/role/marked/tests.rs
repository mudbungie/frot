//! Each case as Chrome 158 has it (`bl-0725`); the corpus page `roles.html`
//! holds the same ones against the recorded capture.

use crate::ax::tree::ax_tree;
use crate::dom::Document;
use serde_json::Value;

fn roles(v: &Value, out: &mut Vec<String>) {
    for node in v.as_array().unwrap() {
        let name = node["name"]
            .as_str()
            .map_or(String::new(), |n| format!(" {n:?}"));
        out.push(format!("{}{name}", node["role"].as_str().unwrap()));
        roles(&node["children"], out);
    }
}

fn outline(html: &str) -> String {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    let mut out = Vec::new();
    roles(&ax_tree(&doc, Some(&styles)), &mut out);
    out.join(", ")
}

#[test]
fn autofocus_or_draggable_makes_a_kept_plain_container_a_group() {
    for html in [
        "<div autofocus>text</div>",
        "<div draggable=true>text</div>",
        "<div draggable=false>text</div>",
        "<div autofocus=false>text</div>",
        "<p>a <span id=s autofocus>x</span></p>",
        "<p>a <span id='' draggable>x</span></p>",
        "<p>a <span tabindex=0 autofocus>x</span></p>",
        "<p>a <span contenteditable autofocus>x</span></p>",
        "<p>a <b id=b autofocus>x</b></p>",
        "<b autofocus style='display:block'>x</b>",
        "<pre autofocus>x</pre>",
        "<div autofocus style='display:inline-block'>x</div>",
        "<div id=d autofocus><p></p></div>",
    ] {
        assert!(outline(html).contains("group"), "{html}");
    }
    assert_eq!(
        outline("<p>a <span autofocus aria-label=N>x</span></p>"),
        "paragraph, group \"N\""
    );
    assert_eq!(
        outline("<p>a <span autofocus title=T>x</span></p>"),
        "paragraph, group \"T\""
    );
}

#[test]
fn a_container_chrome_does_not_keep_is_no_group() {
    for html in [
        // A span is never kept for its box.
        "<p>a <span autofocus>x</span></p>",
        "<div><span autofocus style='display:block'>x</span></div>",
        "<p>a <b draggable=true>x</b></p>",
        // A block with no line in it.
        "<div autofocus><h1>h</h1></div>",
        "<div autofocus></div>",
        "<div autofocus style='display:inline'>x</div>",
    ] {
        assert!(!outline(html).contains("group"), "{html}");
    }
}

#[test]
fn nothing_else_makes_one_and_a_role_turns_it_off() {
    for html in [
        "<div>text</div>",
        "<p>a <span id=s tabindex=0>x</span></p>",
        "<p>a <span id=s title=t onclick=1>x</span></p>",
        "<div autofocus role=bogus>x</div>",
        "<div autofocus role=generic>x</div>",
        "<div autofocus role=presentation>x</div>",
        // Generic on purpose, or not a plain container at all.
        "<section autofocus>x</section>",
        "<p>a <a id=a autofocus>x</a></p>",
        "<summary id=s autofocus>x</summary>",
        "<svg><g id=g autofocus><text>t</text></g></svg>",
    ] {
        assert!(!outline(html).contains("group"), "{html}");
    }
    // An element with a role keeps it.
    assert_eq!(
        outline("<p>a <em id=e autofocus>x</em></p>"),
        "paragraph, emphasis"
    );
    assert_eq!(outline("<div autofocus role=note>x</div>"), "note");
}

/// `bl-d61c`: a plain list-item box is a list item; an element with a role
/// of its own keeps it.
#[test]
fn a_plain_list_item_box_is_a_list_item() {
    for (html, want) in [
        ("<span style='display:list-item'>x</span>", "listitem"),
        ("<span style='display:list-item'></span>", "listitem"),
        ("<b style='display:list-item'>b</b>", "listitem"),
        (
            "<div style='display:list-item;list-style:none'>n</div>",
            "listitem",
        ),
        (
            "<div autofocus style='display:list-item'>t</div>",
            "listitem",
        ),
        ("<p style='display:list-item'>p</p>", "paragraph"),
        ("<h3 style='display:list-item'>h</h3>", "heading \"h\""),
        ("<div role=none style='display:list-item'>r</div>", ""),
        (
            "<div role=foo style='display:list-item'>f</div>",
            "listitem",
        ),
        ("<section style='display:list-item'>s</section>", ""),
    ] {
        assert_eq!(outline(html), want, "{html}");
    }
}

/// `bl-824c`, Chrome 158: a custom element is plain, a click handler holds a
/// node, and a table-cell box is no group.
#[test]
fn custom_elements_click_handlers_and_table_cells() {
    for (html, want) in [
        ("<x-el autofocus style='display:block'>x</x-el>", "group"),
        (
            "<x-el draggable=true style='display:block'>x</x-el>",
            "group",
        ),
        ("<x-el autofocus>inline</x-el>", ""),
        ("<x-el id=q style='display:block'>unmarked</x-el>", ""),
        ("<x-el style='display:list-item'>l</x-el>", "listitem"),
        ("<span autofocus onclick=1>c</span>", "group"),
        ("<span autofocus onmousedown=1>c</span>", "group"),
        ("<span autofocus onmouseup=''>c</span>", "group"),
        ("<span autofocus onkeydown=1>c</span>", ""),
        ("<span autofocus onfocus=1>c</span>", ""),
        ("<p onclick=1></p>", "paragraph"),
        ("<p onkeydown=1></p>", ""),
        ("<div autofocus style='display:table-cell'>tc</div>", ""),
        (
            "<div autofocus style='display:inline-block'>ib</div>",
            "group",
        ),
        ("<div autofocus style='display:flex'>fl</div>", ""),
    ] {
        assert_eq!(outline(html), want, "{html}");
    }
}

/// A `popover` makes a plain container a group whatever its box (`bl-cf7b`);
/// a never-shown one has no box unless a stylesheet gives it one.
#[test]
fn popover_makes_a_plain_container_a_group() {
    for (html, want) in [
        ("<div popover style='display:block'>x</div>", "group"),
        (
            "<p>a <span popover style='display:inline'>x</span></p>",
            "paragraph, group",
        ),
        ("<div popover>x</div>", ""),
        ("<div popover role=none style='display:block'>x</div>", ""),
        ("<section popover style='display:block'>x</section>", ""),
        (
            "<ul popover style='display:block'><li>x</li></ul>",
            "list, listitem",
        ),
    ] {
        assert_eq!(outline(html), want, "{html}");
    }
}
