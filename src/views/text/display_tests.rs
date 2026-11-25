//! Line breaks by `display` value, as Chrome 158 `innerText` has them
//! (`bl-e3a0`). The corpus page `display.html` holds the same cases against
//! the recorded capture.

use super::tests::{t, t_css};

fn one(display: &str) -> String {
    t_css(&format!(
        "<div>p<span style='display:{display}'>ct</span>q</div>"
    ))
}

#[test]
fn a_box_that_is_not_block_level_stays_on_its_line() {
    for display in [
        "contents",
        "inline-grid",
        "inline-table",
        "ruby",
        "ruby-text",
        "-webkit-inline-box",
        "inline flex",
        "table-cell",
        "table-row",
        "table-row-group",
        // Not a `display` value: the span stays inline.
        "run-in",
        "bogus",
    ] {
        assert_eq!(one(display), "pctq", "{display}");
    }
    for display in ["table", "table-caption", "grid", "flow-root", "-webkit-box"] {
        assert_eq!(one(display), "p\nct\nq", "{display}");
    }
    // An invalid value leaves a div the block it was.
    assert_eq!(
        t_css("<div>p<div style='display:run-in'>ct</div>q</div>"),
        "p\nct\nq"
    );
}

#[test]
fn a_cell_is_set_off_from_the_cell_before_it_and_a_row_from_the_row() {
    let cell = |s: &str| format!("<span style='display:table-cell'>{s}</span>");
    let row = |s: &str| format!("<span style='display:table-row'>{s}</span>");
    let html = format!("<div>ia{}{}id</div>", cell("ib"), cell("ic"));
    assert_eq!(t_css(&html), "iaib icid");
    let html = format!("<div>ja{}{}jd</div>", row("jb"), row("jc"));
    assert_eq!(t_css(&html), "jajb\njcjd");
    // White space between two cells is not content between them.
    let html = format!("<div>la{} {}ld</div>", cell("lb"), cell("lc"));
    assert_eq!(t_css(&html), "lalb lcld");
    // Text, another element, or the end of the element around it is.
    let html = format!("<div>ra{}rc{}re</div>", cell("rb"), cell("rd"));
    assert_eq!(t_css(&html), "rarbrcrdre");
    let html = format!("<div>ka{}<img alt=''>{}kd</div>", cell("kb"), cell("kc"));
    assert_eq!(t_css(&html), "kakbkckd");
    let html = format!("<div>na<b>{}</b>{}nd</div>", cell("nb"), cell("nc"));
    assert_eq!(t_css(&html), "nanbncnd");
    // A row before a bare cell is set off from the cell's anonymous row
    // (Chrome 158, `bl-34f1`; this read "mambmcmd" before).
    let html = format!("<div>ma{}{}md</div>", row("mb"), cell("mc"));
    assert_eq!(t_css(&html), "mamb\nmcmd");
    // Preformatted text is content too.
    let html = format!("<pre>{}\n{}</pre>", cell("a"), cell("b"));
    assert_eq!(t_css(&html), "a\nb");
}

#[test]
fn a_css_table_reads_row_by_row() {
    let html = "<style>.t{display:table}.h{display:table-header-group}\
                .g{display:table-row-group}.r{display:table-row}.c{display:table-cell}</style>\
                x<div class=t><div class=h><div class=r><i class=c>a</i><i class=c>b</i></div></div>\
                <div class=g><div class=r><i class=c>c</i> <i class=c>d</i></div></div></div>y";
    assert_eq!(t_css(html), "x\na b\nc d\ny");
}

#[test]
fn a_paragraph_with_no_box_or_a_table_parts_is_on_no_line_of_its_own() {
    let p = |display: &str| t_css(&format!("<div>x<p style='display:{display}'>pp</p>y</div>"));
    for display in ["contents", "table-cell", "table-row"] {
        assert_eq!(p(display), "xppy", "{display}");
    }
    for display in ["inline", "inline-grid", "ruby", "inline-flex"] {
        assert_eq!(p(display), "x\npp\ny", "{display}");
    }
}

#[test]
fn a_box_out_of_flow_is_a_block_unless_it_is_no_box() {
    let out = |display: &str| {
        t_css(&format!(
            "<div>a<span style='position:absolute;display:{display}'>ct</span>b</div>"
        ))
    };
    for display in ["table-cell", "table-row", "inline-grid", "ruby"] {
        assert_eq!(out(display), "a\nct\nb", "{display}");
    }
    assert_eq!(out("contents"), "actb");
}

#[test]
fn generated_content_follows_its_own_display() {
    let g = |display: &str| {
        t_css(&format!(
            "<style>i::before{{content:'B';display:{display}}}</style><div>g1<i>g2</i>g3</div>"
        ))
    };
    for display in ["table-cell", "contents", "inline-table", "table-row"] {
        assert_eq!(g(display), "g1Bg2g3", "{display}");
    }
    assert_eq!(g("grid"), "g1\nB\ng2g3");
}

/// The tags Chrome 158's UA sheet makes block-level that `tags::BLOCK_TAGS`
/// had missed (`bl-bccf`): each stands on a line of its own, with a cascade
/// and without one.
#[test]
fn a_legend_and_the_other_ua_blocks_stand_on_their_own_line() {
    let fieldset = "<fieldset disabled><legend>Off</legend><button>off</button></fieldset>";
    assert_eq!(t_css(fieldset), "Off\noff");
    assert_eq!(t(fieldset), "Off\noff");
    for tag in [
        "legend", "center", "dir", "hgroup", "listing", "menu", "search", "xmp",
    ] {
        let html = format!("<div>a<{tag}>b</{tag}>c</div>");
        assert_eq!(t_css(&html), "a\nb\nc", "{tag}");
        assert_eq!(t(&html), "a\nb\nc", "{tag}");
    }
    assert_eq!(t_css("<div>a<plaintext>b"), "a\nb");
    // `<marquee>` is an inline-block.
    assert_eq!(t_css("<div>a<marquee>b</marquee>c</div>"), "abc");
}

/// Separators follow Chrome's anonymous table boxes (`bl-34f1`, Chrome 158;
/// `tests/fixtures/fidelity/tableparts.html` holds the same cases).
#[test]
fn table_parts_are_separated_by_their_anonymous_table() {
    let cell = |s: &str| format!("<span style='display:table-cell'>{s}</span>");
    let row = |s: &str| format!("<span style='display:table-row'>{s}</span>");
    let rg = |s: &str| format!("<span style='display:table-row-group'>{s}</span>");
    for (html, want) in [
        (
            format!("m1{}{}{}m5", cell("m2"), row("m3"), cell("m4")),
            "m1m2m3\nm4m5",
        ),
        (format!("{}{}", rg("ka"), rg("kb")), "kakb"),
        (format!("{}{}", rg(&row("ga")), rg(&row("gb"))), "ga\ngb"),
        (format!("{}{}", row("ha"), rg("hb")), "ha\nhb"),
        (row(&(cell("d1") + "d2")), "d1 d2"),
        (format!("{}<b>e2</b>{}", cell("e1"), cell("e3")), "e1e2e3"),
        (format!("{}{}", rg(&row("f1")), row("f2")), "f1\nf2"),
        (format!("s1 {} s3", cell("s2")), "s1s2s3"),
        (format!("{}<br>x", cell("c")), "c\nx"),
        (rg(&(rg("a") + "b")), "ab"),
    ] {
        assert_eq!(t_css(&format!("<div>{html}</div>")), want, "{html}");
    }
    let html = "<div><p style='display:contents'>p1</p> <p style='display:table-cell'>p2</p></div>";
    assert_eq!(t_css(html), "p1p2");
}
