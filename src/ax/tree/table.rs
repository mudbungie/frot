//! Chromium's data-table test (`IsDataTable`), measured in Chrome 158
//! (`bl-dfc3`, `bl-7f3a`, `bl-e143`): whether a `<table>` is *layout* — its
//! table parts collapse out of the tree (module docs of [`super`]).

use crate::css::{Paint, Styles};
use crate::dom::{Document, Element, NodeId, NodeKind};

/// `true` (demote to presentation) when a `<table>` carries none of the
/// data-table signals in [`super`]'s module docs.
pub fn is_layout_table(doc: &Document, id: NodeId, el: &Element, styles: Option<&Styles>) -> bool {
    let said = |a: &str| el.attr(a).is_some_and(|s| !s.is_empty());
    let structure = ["caption", "thead", "tfoot", "colgroup", "col"];
    let child = doc.node(id).children.iter().any(|&c| {
        matches!(&doc.node(c).kind, NodeKind::Element(e) if structure.contains(&e.name.as_str()))
    });
    let signed = said("summary") || said("rules") || el.attr("role").is_some() || child;
    if signed || crate::css::editable(doc, id, el) {
        return false;
    }
    let (rows, cells) = (parts(doc, id, &["tr"]), parts(doc, id, &["td", "th"]));
    let marked = |&(_, c): &(NodeId, &Element)| {
        c.name == "th"
            || ["headers", "abbr", "axis", "scope"]
                .iter()
                .any(|a| c.attr(a).is_some_and(|v| !v.is_empty()))
    };
    if rows.len() == 1 && cells.len() == 1 {
        return true;
    }
    let data = rows.len() >= 20
        || cells.iter().any(marked)
        || (cells.len() > 1 && bordered(el))
        || styles.is_some_and(|s| painted(s, id, &rows, &cells));
    !data
}

/// The signals Chrome reads off the cascade (`bl-e143`): a cell with
/// `empty-cells: hide`; at least half the cells (rounded down) drawn with
/// borders — top and bottom, or left and right, or any one side alike — or
/// with a background other than the table's; or the first rows' backgrounds
/// alternating. A cell with no box is not counted.
fn painted(
    s: &Styles,
    table: NodeId,
    rows: &[(NodeId, &Element)],
    cells: &[(NodeId, &Element)],
) -> bool {
    let cells: Vec<&Paint> = cells
        .iter()
        .filter(|(c, _)| !s.display_none(*c))
        .map(|(c, _)| s.paint(*c))
        .collect();
    if cells.iter().any(|p| p.empty_cells_hide) {
        return true;
    }
    let half = cells.len() / 2;
    let count = |f: &dyn Fn(&Paint) -> bool| cells.iter().filter(|p| f(p)).count();
    let framed = |p: &Paint| {
        let [t, r, b, l] = p.borders;
        (t && b) || (l && r)
    };
    let back = &s.paint(table).background;
    let differs = |p: &Paint| p.background.is_some() && &p.background != back;
    let side = |i: usize| count(&move |p: &Paint| p.borders[i]) >= half;
    cells.len() > 1 && (count(&framed) >= half || (0..4).any(side) || count(&differs) >= half)
        || alternating(s, rows)
}

/// Whether the first five rows' backgrounds alternate: three rows or more,
/// each even one coloured as the first and each odd one not.
fn alternating(s: &Styles, rows: &[(NodeId, &Element)]) -> bool {
    let colors: Vec<&Option<String>> = rows
        .iter()
        .take(5)
        .map(|(r, _)| &s.paint(*r).background)
        .collect();
    colors.len() > 2
        && colors
            .iter()
            .enumerate()
            .all(|(k, c)| (k % 2 == 0) == (*c == colors[0]))
}

/// Whether the `border` attribute draws cell borders: present, and not a
/// non-negative integer equal to zero — an unparsable value means 1.
fn bordered(el: &Element) -> bool {
    el.attr("border").is_some_and(|v| {
        let digits: String = v
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.is_empty() || digits.bytes().any(|b| b != b'0')
    })
}

/// The `tags` elements in the subtree, without descending into nested
/// `<table>` elements — an inner data table's header must not rescue an outer
/// layout table, and its cells are not the outer table's.
fn parts<'d>(doc: &'d Document, id: NodeId, tags: &[&str]) -> Vec<(NodeId, &'d Element)> {
    let mut found = Vec::new();
    collect(doc, id, tags, &mut found);
    found
}

fn collect<'d>(
    doc: &'d Document,
    id: NodeId,
    tags: &[&str],
    found: &mut Vec<(NodeId, &'d Element)>,
) {
    for &c in &doc.node(id).children {
        match &doc.node(c).kind {
            NodeKind::Element(e) if tags.contains(&e.name.as_str()) => found.push((c, e)),
            NodeKind::Element(e) if e.name != "table" => collect(doc, c, tags, found),
            _ => {}
        }
    }
}
