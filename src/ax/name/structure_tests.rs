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
/// Chrome also spaces them — "G1 10" — as does frot (`bl-fb69`, below).
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

// --- Separators follow the box (`bl-fb69`) ---------------------------------
//
// Every expectation is Chrome 158's name for the same markup
// (`Accessibility.getFullAXTree`, `--headless=new`).

#[test]
fn a_block_child_is_spaced_off_and_an_inline_one_is_not() {
    for (html, want) in [
        (
            "<button>A<div>blk1</div><div>blk2</div>B</button>",
            "A blk1 blk2 B",
        ),
        (
            "<button><span>in1</span><span>in2</span></button>",
            "in1in2",
        ),
        ("<button>e<div></div>f</button>", "e f"),
        ("<button>n<div><div>nest</div></div>o</button>", "n nest o"),
        (
            "<button>li<ul><li>one</li><li>two</li></ul></button>",
            "li one two",
        ),
        ("<button>hr<hr>rule</button>", "hr rule"),
    ] {
        assert_eq!(name(html, "button").as_deref(), Some(want), "{html}");
        assert_eq!(name_css(html, "button").as_deref(), Some(want), "{html}");
    }
}

#[test]
fn a_line_break_separates_words() {
    assert_eq!(name("<h2>head<br>br</h2>", "h2"), Some("head br".into()));
    assert_eq!(name("<h2>wb<wbr>wb2</h2>", "h2"), Some("wb wb2".into()));
}

#[test]
fn a_table_cell_boundary_is_a_space_and_an_inline_gridcell_is_not() {
    let cells = "<table role=grid><tr><th scope=row>G1</th><td>10</td></tr></table>";
    assert_eq!(name(cells, "tr"), Some("G1 10".into()));
    let spans = "<div role=grid><div role=row id=r><span role=gridcell>a</span>\
                 <span role=gridcell>b</span></div></div>";
    let d = Document::parse(spans);
    let row = d.element_by_id("r").unwrap();
    assert_eq!(accessible_name(&d, row, None), Some("ab".into()));
}

/// Under `--css` the box is the computed `display`, whatever the tag.
#[test]
fn the_computed_display_decides_under_css() {
    for (html, want) in [
        (
            "<button>x<span style='display:block'>sb</span>y</button>",
            "x sb y",
        ),
        (
            "<button>x<div style='display:inline'>di</div>y</button>",
            "xdiy",
        ),
        (
            "<button>x<span style='display:inline-block'>ib</span>y</button>",
            "x ib y",
        ),
        (
            "<button>x<span style='display:flex'>fx</span>y</button>",
            "x fx y",
        ),
        (
            "<button>x<span style='display:inline-flex'>ifx</span>y</button>",
            "x ifx y",
        ),
        (
            "<button>x<span style='display:list-item'>li</span>y</button>",
            "x li y",
        ),
    ] {
        assert_eq!(name_css(html, "button").as_deref(), Some(want), "{html}");
    }
    // Without the cascade the tag's own box stands.
    let html = "<button>x<div style='display:inline'>di</div>y</button>";
    assert_eq!(name(html, "button"), Some("x di y".into()));
}

#[test]
fn a_label_and_a_reference_space_their_blocks_too() {
    let label = "<label for=z>la<div>lb</div>lc</label><input id=z>";
    assert_eq!(name(label, "input"), Some("la lb lc".into()));
    let reference = "<button aria-labelledby=r>zz</button><div id=r>ra<div>rb</div>rc</div>";
    assert_eq!(name(reference, "button"), Some("ra rb rc".into()));
}

/// `bl-2b09`, Chrome 158: a table without a caption is named by `summary`.
#[test]
fn a_table_is_named_from_its_summary_when_it_has_no_caption() {
    let rows = "<tr><th>a</th><th>b</th></tr><tr><td>1</td><td>2</td></tr>";
    for (open, want) in [
        ("<table summary=SUM>", Some("SUM")),
        ("<table summary=SUM><caption>CAP</caption>", Some("CAP")),
        ("<table summary=SUM><caption></caption>", None),
        ("<table summary=SUM aria-label=AL>", Some("AL")),
        ("<table summary=SUM title=TT>", Some("SUM")),
        ("<table summary=''>", None),
        ("<table summary=SUM role=grid>", Some("SUM")),
        ("<table>", None),
    ] {
        let html = format!("{open}{rows}</table>");
        assert_eq!(name(&html, "table").as_deref(), want, "{open}");
    }
    let html = "<div role=table summary=S><div role=row><span role=cell>x</span></div></div>";
    assert_eq!(name(html, "div"), None);
}
