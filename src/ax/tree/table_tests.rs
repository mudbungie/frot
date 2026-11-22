//! The layout-table test's two measured edges (`bl-dfc3`, Chrome 158): the
//! one-cell table and the empty `summary`.

use super::*;

fn has_table(html: &str) -> bool {
    let v = ax_tree(&Document::parse(html), None);
    v.as_array()
        .expect("roots")
        .iter()
        .any(|n| n["role"] == "table")
}

#[test]
fn a_one_cell_table_is_layout_unless_captioned_or_summarised() {
    for html in [
        "<table><tr><th>x</th></tr></table>",
        "<table title=T><tr><th>x</th></tr></table>",
        "<table summary=''><tr><th>x</th></tr></table>",
        "<table aria-label=L><tr><td>x</td></tr></table>",
        "<table aria-labelledby=q><tr><td>x</td></tr></table>",
    ] {
        assert!(!has_table(html), "{html}");
    }
    for html in [
        "<table summary=S><tr><td>x</td></tr></table>",
        "<table><caption>c</caption><tr><td>x</td></tr></table>",
        "<table role=table><tr><td>x</td></tr></table>",
        // One cell, two rows; one row, two cells.
        "<table><tr><th>x</th></tr><tr></tr></table>",
        "<table><tr><th>x</th><td>y</td></tr></table>",
        "<table><tr><th>x</th></tr><tr><td>y</td></tr></table>",
    ] {
        assert!(has_table(html), "{html}");
    }
    // A nested table's cells are its own: the outer one has a single cell.
    let nested = "<table><tr><td><table><tr><th>x</th><td>y</td></tr></table></td></tr></table>";
    let v = ax_tree(&Document::parse(nested), None);
    assert_eq!(v.as_array().expect("roots").len(), 1);
    assert_eq!(v[0]["children"][0]["children"][0]["role"], "rowheader");
}

#[test]
fn an_empty_summary_is_no_signal() {
    let rows = "<tr><td>x</td><td>y</td></tr><tr><td>x</td><td>y</td></tr>";
    assert!(!has_table(&format!("<table summary=''>{rows}</table>")));
    assert!(has_table(&format!("<table summary=' '>{rows}</table>")));
}
