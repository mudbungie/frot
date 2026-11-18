//! Names of structural roles — tables and their parts, description lists —
//! as the fidelity oracle (`docs/design/fidelity.md`) recorded them.
//!
//! Every case is a Chrome 156 measurement (`Accessibility.getFullAXTree`,
//! `--headless=new`, 2026-11-18): the first of each group is in the oracle's
//! corpus, the rest were measured for the ball that fixed the group.

use super::*;

fn name(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), None)
}

fn name_css(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    let s = crate::css::compute(&d);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), Some(&s))
}

/// `bl-fd85`: a `<table>` is named by its first `<caption>` child.
#[test]
fn a_table_is_named_from_its_caption() {
    for (html, want) in [
        // The corpus case (`structure.html`).
        (
            "<table><caption>Quarterly totals</caption><tr><th>Q</th></tr></table>",
            Some("Quarterly totals"),
        ),
        // Wherever among the children the caption sits.
        (
            "<table><tr><th>x</th></tr><caption>late</caption></table>",
            Some("late"),
        ),
        // The first of two.
        (
            "<table><caption>first</caption><caption>second</caption><tr><th>x</th></tr></table>",
            Some("first"),
        ),
        // The caption's text alternatives, not its raw text.
        (
            "<table><caption><img alt=IMG> tail</caption><tr><th>x</th></tr></table>",
            Some("IMG tail"),
        ),
        // The caption's own `aria-label` speaks for it …
        (
            "<table><caption aria-label=CAL>text</caption><tr><th>x</th></tr></table>",
            Some("CAL"),
        ),
        // … but its `title` does not: a caption is read as a descendant.
        (
            "<table><caption title=CT></caption><tr><th>x</th></tr></table>",
            None,
        ),
        // An excluded caption names nothing.
        (
            "<table><caption aria-hidden=true>ah</caption><tr><th>x</th></tr></table>",
            None,
        ),
        // No caption, no name; an empty one likewise.
        ("<table><tr><th>x</th></tr></table>", None),
        (
            "<table><caption></caption><tr><th>x</th></tr></table>",
            None,
        ),
        // The author's name outranks the caption; the caption outranks `title`;
        // a reference that resolves to nothing falls through to it.
        (
            "<table aria-label=AL><caption>cap</caption><tr><th>x</th></tr></table>",
            Some("AL"),
        ),
        (
            "<table title=TI><caption>cap</caption><tr><th>x</th></tr></table>",
            Some("cap"),
        ),
        (
            "<table aria-labelledby=nope><caption>cap</caption><tr><th>x</th></tr></table>",
            Some("cap"),
        ),
        // An author role on the table does not change where its name is.
        (
            "<table role=grid><caption>gridcap</caption><tr><th>x</th></tr></table>",
            Some("gridcap"),
        ),
    ] {
        assert_eq!(name(html, "table").as_deref(), want, "{html}");
    }
}

/// A caption CSS hides names nothing — `display:none` and `visibility:hidden`
/// alike (Chrome 156: `table ""` for both).
#[test]
fn a_hidden_caption_names_nothing() {
    for style in ["display:none", "visibility:hidden"] {
        let html =
            format!("<table><caption style='{style}'>hid</caption><tr><th>x</th></tr></table>");
        assert_eq!(name_css(&html, "table"), None, "{style}");
    }
}

/// `bl-3072`: a row of a plain `table` has no name of its own — focusable or
/// not, `<tr>` or `role=row` — and still takes the author's.
#[test]
fn a_table_row_is_not_named_from_its_cells() {
    for (html, want) in [
        // The corpus case (`structure.html`): frot said "Q110".
        (
            "<table><caption>c</caption><tr><th scope=row>Q1</th><td>10</td></tr></table>",
            None,
        ),
        (
            "<table><caption>c</caption><tr tabindex=0><th scope=row>Q2</th><td>12</td></tr></table>",
            None,
        ),
        (
            "<table><caption>c</caption><tr aria-label=RL><td>z</td></tr></table>",
            Some("RL"),
        ),
        (
            "<table><caption>c</caption><tr title=RT><td>z</td></tr></table>",
            Some("RT"),
        ),
    ] {
        assert_eq!(name(html, "tr").as_deref(), want, "{html}");
    }
    let html = "<div role=table><p role=row><span role=cell>a</span></p></div>";
    assert_eq!(name(html, "p"), None);
}

/// In a `grid` or `treegrid` the row *is* named from its cells (Chrome 156:
/// `row "ab"` for the inline cells below). Where the cells are table cells
/// Chrome also spaces them — "G1 10" — which frot does not yet (`bl-fb69`).
#[test]
fn a_grid_row_is_named_from_its_cells() {
    for grid in ["grid", "treegrid"] {
        let html = format!(
            "<div role={grid}><p role=row><span role=gridcell>a</span>\
             <span role=gridcell>b</span></p></div>"
        );
        assert_eq!(name(&html, "p").as_deref(), Some("ab"), "{grid}");
        let html = format!("<table role={grid}><tr><td>G2</td></tr></table>");
        assert_eq!(name(&html, "tr").as_deref(), Some("G2"), "{grid}");
    }
}

/// A node with no role at all names nothing from its contents.
#[test]
fn a_roleless_node_has_no_name_from_contents() {
    assert_eq!(name("<video>fallback</video>", "video"), None);
}

/// `bl-8ad7`: a `term` is named from its contents, whichever tag carries the
/// role; a `definition` is not.
#[test]
fn a_term_is_named_from_its_contents() {
    for (html, tag, want) in [
        // The corpus case (`structure.html`).
        (
            "<dl><dt>Term</dt><dd>Definition text</dd></dl>",
            "dt",
            Some("Term"),
        ),
        ("<dl><dt>Term</dt><dd>Definition text</dd></dl>", "dd", None),
        ("<dl><dt><img alt=IM> tail</dt></dl>", "dt", Some("IM tail")),
        (
            "<dl><dt>a <a href=x>lnk</a> b</dt></dl>",
            "dt",
            Some("a lnk b"),
        ),
        ("<dl><dt aria-label=AL>x</dt></dl>", "dt", Some("AL")),
        ("<dl><dt></dt></dl>", "dt", None),
        ("<dt>orphan</dt>", "dt", Some("orphan")),
        ("<p><dfn>dfnterm</dfn></p>", "dfn", Some("dfnterm")),
        ("<p><dfn title=DT>dfn2</dfn></p>", "dfn", Some("dfn2")),
        (
            "<p><span role=term>spanterm</span></p>",
            "span",
            Some("spanterm"),
        ),
        ("<p><span role=definition>spandef</span></p>", "span", None),
    ] {
        assert_eq!(name(html, tag).as_deref(), want, "{html}");
    }
}
