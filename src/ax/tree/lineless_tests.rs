//! The lineless `<p>` and `<caption>` ([`super::lineless`]): every case is a Chrome 158
//! measurement (`bl-5c70`), the roles listed being the page's whole outline.

use super::*;

fn roles(v: &Value, out: &mut Vec<String>) {
    for n in v.as_array().expect("children") {
        out.push(n["role"].as_str().expect("role").to_string());
        roles(&n["children"], out);
    }
}

/// The roles `html` emits, in order, with and without a cascade.
fn outline(html: &str, css: bool) -> String {
    let doc = Document::parse(html);
    let styles = css.then(|| crate::css::compute(&doc));
    let mut out = Vec::new();
    roles(&ax_tree(&doc, styles.as_ref()), &mut out);
    out.join(" ")
}

const BLOCK: &str = "<span style='display:block'>x</span>";

#[test]
fn a_paragraph_with_no_line_has_no_node() {
    for html in [
        "<p></p>",
        "<p> \n\t</p>",
        "<p><!--c--></p>",
        "<p><script>1</script><style>a{}</style><template>x</template></p>",
    ] {
        assert_eq!(outline(html, true), "", "{html}");
        assert_eq!(outline(html, false), "", "{html}");
    }
    for html in [
        "<p><span style='display:none'>x</span> </p>",
        "<p><span hidden>x</span></p>",
        "<p><input type=hidden></p>",
    ] {
        assert_eq!(outline(html, true), "", "{html}");
    }
}

#[test]
fn a_paragraph_with_a_block_level_child_has_no_node() {
    for rest in ["", "text", "<!--c-->", "<b>inline</b>", " "] {
        let html = format!("<p>{BLOCK}{rest}</p>");
        assert_eq!(outline(&html, true), "", "{html}");
    }
    // The content is still there, in the paragraph's place.
    let html = "<p><a href=x style='display:block'>l</a></p>";
    assert_eq!(outline(html, true), "link");
    assert_eq!(
        outline("<p><span style='display:flex'>x</span></p>", true),
        ""
    );
    // Without a cascade the box is the tag's own.
    assert_eq!(outline("<p><option>o</option></p>", false), "option");
    assert_eq!(outline(&format!("<p>{BLOCK}</p>"), false), "paragraph");
}

#[test]
fn a_paragraph_that_is_no_block_container_has_no_node() {
    for display in ["inline", "flex", "inline-flex"] {
        let html = format!("<p style='display:{display}'>text</p>");
        assert_eq!(outline(&html, true), "", "{html}");
    }
    assert_eq!(
        outline("<p style='display:inline'>text</p>", false),
        "paragraph"
    );
}

/// Measured in Chrome 158 (`bl-e3a0`).
#[test]
fn contents_and_table_parts_are_read_as_the_boxes_they_are() {
    // No box, and a table row's, are no block container.
    for display in ["contents", "table-row", "ruby"] {
        let html = format!("<p style='display:{display}'>text</p>");
        assert_eq!(outline(&html, true), "", "{html}");
    }
    // A table cell is one, and an inline-level table or grid too.
    for display in ["table-cell", "inline-grid", "inline-table"] {
        let html = format!("<p style='display:{display}'>text</p>");
        assert_eq!(outline(&html, true), "paragraph", "{html}");
    }
    assert_eq!(outline("<p style='display:table-cell'></p>", true), "");
    // A table part inside is block-level; an inline-level table is a line.
    for display in ["table-cell", "table-row"] {
        let html = format!("<p><span style='display:{display}'>x</span></p>");
        assert_eq!(outline(&html, true), "", "{html}");
    }
    let html = "<p><span style='display:inline-table'>x</span></p>";
    assert_eq!(outline(html, true), "paragraph");
    // `contents` is looked through: its children are the paragraph's.
    let through = |inner: &str| format!("<p><span style='display:contents'>{inner}</span></p>");
    assert_eq!(outline(&through("x"), true), "paragraph");
    assert_eq!(outline(&through(BLOCK), true), "");
    assert_eq!(outline(&through(""), true), "");
    assert_eq!(outline(&through("<b>i</b><!--c-->"), true), "paragraph");
}

#[test]
fn any_inline_level_box_is_a_line() {
    for html in [
        "<p>text</p>",
        "<p>&nbsp;</p>",
        "<p><span></span></p>",
        "<p><span> </span></p>",
        "<p><span style='visibility:hidden'>x</span></p>",
        "<p><span style='display:inline-block'>x</span></p>",
        "<p><span style='display:inline-flex'>x</span></p>",
        "<p><br></p>",
        "<p style='display:inline-block'>text</p>",
        "<p style='display:list-item'></p>",
        "<style>p::before{content:'g'}</style><p></p>",
        "<style>p::after{content:'g'}</style><p></p>",
    ] {
        assert_eq!(outline(html, true), "paragraph", "{html}");
    }
    // A block inside an inline child is the child's, not the paragraph's.
    let html = format!("<p><a href=x>{BLOCK}</a></p>");
    assert_eq!(outline(&html, true), "paragraph link");
}

#[test]
fn the_author_can_hold_a_lineless_paragraph() {
    for attr in [
        "id=i",
        "id=''",
        "aria-label=L",
        "aria-label=''",
        "aria-describedby=q",
        "title=T",
        "tabindex=0",
        "contenteditable",
        "role=paragraph",
        "role='foo paragraph'",
    ] {
        for body in ["", BLOCK] {
            let html = format!("<p {attr}>{body}</p>");
            assert_eq!(outline(&html, true), "paragraph", "{html}");
        }
    }
    for attr in ["title=''", "class=c", "lang=en", "role=foo"] {
        let html = format!("<p {attr}></p>");
        assert_eq!(outline(&html, true), "", "{html}");
    }
}

/// Only `<p>` has the rule: an explicit paragraph, and every other role, keeps
/// an empty node.
#[test]
fn the_rule_is_the_p_elements_alone() {
    let html = format!("<div role=paragraph></div><div role=paragraph>{BLOCK}</div>");
    assert_eq!(outline(&html, true), "paragraph paragraph");
    let html = "<h2></h2><blockquote></blockquote><ul><li></li></ul><em></em>";
    assert_eq!(
        outline(html, true),
        "heading blockquote list listitem emphasis"
    );
}

const ROWS: &str = "<tr><th>a</th><th>b</th></tr>";

/// Chrome 158 (`bl-fe88`, `bl-dfc3`): a table `<caption>` has the same rule,
/// and under a cascade it is the caption's *boxes* that decide.
#[test]
fn a_caption_with_no_line_has_no_node() {
    for caption in [
        "<caption></caption>",
        "<caption> \n</caption>",
        "<caption><!-- note --></caption>",
        "<caption title=''></caption>",
        "<caption><span style='display:none'>h</span></caption>",
        "<caption><span style='display:block'>x</span></caption>",
        "<caption style='display:inline'>x</caption>",
    ] {
        let html = format!("<table>{caption}{ROWS}</table>");
        assert_eq!(
            outline(&html, true),
            "table row columnheader columnheader",
            "{html}"
        );
    }
    for caption in [
        "<caption>words</caption>",
        "<caption><span></span></caption>",
        "<caption><img alt=''></caption>",
        "<caption aria-label=CL></caption>",
        "<caption title=CT></caption>",
        "<caption id=x></caption>",
        "<caption role=caption></caption>",
        "<caption class=g></caption>",
    ] {
        let html =
            format!("<style>.g::before{{content:'gen'}}</style><table>{caption}{ROWS}</table>");
        assert!(
            outline(&html, true).starts_with("table caption row"),
            "{html}"
        );
    }
    assert!(
        outline(&format!("<table><caption></caption>{ROWS}</table>"), false)
            .starts_with("table row")
    );
    // Only the table caption: the other two caption-role sources keep theirs.
    let html = "<figure><figcaption></figcaption></figure><div role=caption></div>";
    assert_eq!(outline(html, true), "figure caption caption");
}

/// `bl-346e`, Chrome 158: a float or an absolutely positioned child is
/// blockified and still counts as part of the line; block-level generated
/// content takes the line away.
#[test]
fn out_of_flow_children_and_block_pseudo_elements() {
    for html in [
        "<p><span style='float:left'>fl</span></p>",
        "<p><span style='position:absolute'>ab</span></p>",
        "<p><span style='float:left'>f</span>t</p>",
        "<style>p::before{content:'g';display:inline-block}</style><p></p>",
        "<style>p::before{content:'g';float:left}</style><p>t</p>",
        "<style>p::after{content:'g';position:absolute}</style><p></p>",
    ] {
        assert_eq!(outline(html, true), "paragraph", "{html}");
    }
    for html in [
        "<p><span style='display:block'>x</span><span style='float:left'>f</span></p>",
        "<p><span style='position:absolute;display:none'>x</span></p>",
        "<style>p::after{content:'g';display:block}</style><p></p>",
        "<style>p::after{content:'g';display:block}</style><p>t</p>",
        "<style>p::before{content:'g';float:left} p::after{content:'h';display:flex}</style><p>t</p>",
    ] {
        assert_eq!(outline(html, true), "", "{html}");
    }
}
