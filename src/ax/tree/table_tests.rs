//! The layout-table test, each case measured in Chrome 158 (`bl-dfc3`,
//! `bl-7f3a`): which tables Chrome keeps as `table` and which it demotes.

use super::*;

fn has_table(html: &str) -> bool {
    let v = ax_tree(&Document::parse(html), None);
    v.as_array()
        .expect("roots")
        .iter()
        .any(|n| n["role"] == "table")
}

#[test]
fn table_structure_keeps_even_one_cell() {
    for html in [
        "<table summary=S><tr><td>x</td></tr></table>",
        "<table><caption>c</caption><tr><td>x</td></tr></table>",
        "<table role=table><tr><td>x</td></tr></table>",
        "<table><thead><tr><td>x</td></tr></thead></table>",
        "<table><tfoot><tr><td>x</td></tr></tfoot></table>",
        "<table><colgroup><col></colgroup><tr><td>x</td></tr></table>",
        "<table><col><tr><td>x</td></tr></table>",
        "<table rules=all><tr><td>x</td></tr></table>",
        "<table rules=none><tr><td>x</td></tr></table>",
    ] {
        assert!(has_table(html), "{html}");
    }
    assert!(!has_table(
        "<table rules=''><tr><td>x</td><td>y</td></tr></table>"
    ));
}

#[test]
fn a_one_cell_table_is_layout_whatever_its_cell_says() {
    for html in [
        "<table><tr><th>x</th></tr></table>",
        "<table title=T><tr><th>x</th></tr></table>",
        "<table summary=''><tr><th>x</th></tr></table>",
        "<table><tr><td headers=q>x</td></tr></table>",
        "<table border=1><tr><td>x</td></tr></table>",
    ] {
        assert!(!has_table(html), "{html}");
    }
    // One cell, two rows: a header still keeps it; a border does not, since
    // Chrome wants two cells to call the cells bordered.
    assert!(has_table("<table><tr><th>x</th></tr><tr></tr></table>"));
    assert!(!has_table(
        "<table border=1><tr><td>x</td></tr><tr></tr></table>"
    ));
    // A nested table's cells are its own: the outer one has a single cell.
    let nested = "<table><tr><td><table><tr><th>x</th><td>y</td></tr></table></td></tr></table>";
    let v = ax_tree(&Document::parse(nested), None);
    assert_eq!(v.as_array().expect("roots").len(), 1);
    assert_eq!(v[0]["children"][0]["children"][0]["role"], "rowheader");
}

#[test]
fn cells_carry_the_signals_past_one_cell() {
    for cell in [
        "<th>x</th>",
        "<td headers=q>x</td>",
        "<td scope=row>x</td>",
        "<td abbr=a>x</td>",
        "<td axis=a>x</td>",
    ] {
        assert!(
            has_table(&format!("<table><tr>{cell}<td>y</td></tr></table>")),
            "{cell}"
        );
    }
    assert!(!has_table(
        "<table><tr><td headers=''>x</td><td>y</td></tr></table>"
    ));
    let rows = |n: usize| "<tr><td>r</td></tr>".repeat(n);
    assert!(has_table(&format!("<table>{}</table>", rows(20))));
    assert!(!has_table(&format!("<table>{}</table>", rows(19))));
}

#[test]
fn a_border_that_draws_keeps_two_cells() {
    let two = "<tr><td>x</td><td>y</td></tr>";
    for border in ["1", "2", "", "abc", "-1", " 3"] {
        assert!(
            has_table(&format!("<table border='{border}'>{two}</table>")),
            "{border}"
        );
    }
    for border in ["0", "0px", "00"] {
        assert!(
            !has_table(&format!("<table border='{border}'>{two}</table>")),
            "{border}"
        );
    }
}

#[test]
fn an_author_label_is_no_signal() {
    let rows = "<tr><td>x</td><td>y</td></tr><tr><td>x</td><td>y</td></tr>";
    assert!(!has_table(&format!("<table aria-label=L>{rows}</table>")));
    assert!(!has_table(&format!(
        "<table aria-labelledby=q>{rows}</table>"
    )));
    assert!(!has_table(&format!("<table summary=''>{rows}</table>")));
    assert!(has_table(&format!("<table summary=' '>{rows}</table>")));
}

/// `bl-d61c`: an author role keeps its part of a layout table, and an
/// explicit row keeps the cells in it.
#[test]
fn an_authored_part_of_a_layout_table_stays() {
    let outline = |html: &str| {
        let v = ax_tree(&Document::parse(html), None);
        let mut out = Vec::new();
        let mut walk = vec![v];
        while let Some(Value::Array(ns)) = walk.pop() {
            for n in ns.into_iter().rev() {
                out.push(n["role"].as_str().expect("role").to_string());
                walk.push(n["children"].clone());
            }
        }
        out.join(" ")
    };
    for (html, want) in [
        (
            "<table><tr role=row><td>x</td><td role=cell>y</td></tr></table>",
            "row cell cell",
        ),
        (
            "<table><tr><td role=cell>x</td><td>y</td></tr></table>",
            "cell",
        ),
        ("<table><tr role=row><td>x</td></tr></table>", "row cell"),
        ("<table><tr role=foo><td>x</td></tr></table>", ""),
        (
            "<table><tr><th role=columnheader>h</th></tr></table>",
            "columnheader",
        ),
        (
            "<table><tbody role=rowgroup><tr><td>x</td><td>y</td></tr></tbody></table>",
            "rowgroup",
        ),
        (
            "<table role=none><tr role=row><td>x</td></tr></table>",
            "row",
        ),
    ] {
        assert_eq!(outline(html), want, "{html}");
    }
}

fn has_table_css(html: &str) -> bool {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    let v = ax_tree(&doc, Some(&styles));
    v.as_array()
        .expect("roots")
        .iter()
        .any(|n| n["role"] == "table")
}

/// The signals read off the cascade (`bl-e143`), and `contenteditable`.
#[test]
fn painted_cells_and_editable_tables_are_data() {
    let rows = |style: &str, n: usize| {
        let r: String = (0..n)
            .map(|i| format!("<tr><td>{i}</td><td>v</td></tr>"))
            .collect();
        format!("<style>{style}</style><table>{r}</table>")
    };
    for (html, want) in [
        (rows("td{border:1px solid}", 1), true),
        (rows("td{border-top:1px solid}", 1), true),
        (rows("td:first-child{border:1px solid}", 2), true),
        (
            rows("tr:first-child td:first-child{border-left:1px solid}", 2),
            false,
        ),
        (rows("td{background:red}", 1), true),
        (rows("table,td{background:red}", 1), false),
        (rows("table{empty-cells:hide}", 1), true),
        (rows("tr:nth-child(odd){background:#eee}", 3), true),
        (rows("tr:nth-child(odd){background:#eee}", 2), false),
        (rows("tr{background:#eee}", 5), false),
        (rows("tr:first-child{background:#eee}", 4), false),
        (
            rows("td:first-child{display:none} td{border:1px solid}", 1),
            false,
        ),
        (
            "<table contenteditable><tr><td>x</td><td>y</td></tr></table>".into(),
            true,
        ),
    ] {
        assert_eq!(has_table_css(&html), want, "{html}");
    }
    assert!(has_table(
        "<div contenteditable><table><tr><td>x</td><td>y</td></tr></table></div>"
    ));
    assert!(!has_table(&rows("td{border:1px solid}", 1)));
}
