//! Heading levels, split from the parent module to hold the 300-line cap.
use super::*;

#[test]
fn aria_level_overrides_heading_level() {
    assert_eq!(level(&e("<h2 id=t aria-level=5>x</h2>")), Some(5));
}

#[test]
fn aria_level_zero_or_negative_ignored() {
    assert_eq!(level(&e("<h2 id=t aria-level=0>x</h2>")), Some(2));
    assert_eq!(level(&e("<h2 id=t aria-level=-3>x</h2>")), Some(2));
}

#[test]
fn aria_level_unparseable_ignored() {
    assert_eq!(level(&e("<h2 id=t aria-level=high>x</h2>")), Some(2));
}

#[test]
fn aria_level_applies_to_a_non_heading_tag() {
    assert_eq!(
        level(&e("<div id=t role=heading aria-level=3>x</div>")),
        Some(3)
    );
}

#[test]
fn non_heading_has_no_level() {
    assert_eq!(level(&e("<div id=t>x</div>")), None);
}

// --- The first usable `role=` token (`bl-c5a6`, Chrome 158) ----------------

#[test]
fn an_unknown_token_is_skipped_for_the_next_one() {
    assert_eq!(r("<div id=t role='foo button'>x</div>"), Some("button"));
    assert_eq!(r("<div id=t role='foo bar'>x</div>"), Some("generic"));
}

#[test]
fn an_unnamed_form_or_region_role_does_not_stand() {
    for (html, want) in [
        ("<div id=t role=form><input></div>", "generic"),
        ("<div id=t role=region><p>x</p></div>", "generic"),
        (
            "<div id=t role=region aria-label=''><p>x</p></div>",
            "generic",
        ),
        (
            "<div id=t role=region aria-labelledby=nope><p>x</p></div>",
            "generic",
        ),
        // The next token, then the implicit role, take over.
        ("<div id=t role='form button'>x</div>", "button"),
        ("<div id=t role='region navigation'>x</div>", "navigation"),
        ("<nav id=t role=region>x</nav>", "navigation"),
        ("<h5 id=t role=region>x</h5>", "heading"),
        ("<form id=t role=form></form>", "form"),
        // Named, it stands.
        ("<div id=t role=form aria-label=FL></div>", "form"),
        ("<div id=t role=region title=RT></div>", "region"),
        (
            "<div id=t role=region aria-labelledby=h><h3 id=h>l</h3></div>",
            "region",
        ),
        // Other landmarks need no name.
        ("<div id=t role=navigation>x</div>", "navigation"),
        ("<div id=t role=search>x</div>", "search"),
        ("<div id=t role=group>x</div>", "group"),
    ] {
        assert_eq!(r(html), Some(want), "{html}");
    }
}

/// Chrome 158 (`bl-2ea9`): a table's parts have their roles only while the
/// nearest `<table>` is a table or grid, and an `<li>` only while its list is
/// a list. An author role on the part itself stands.
#[test]
fn a_part_loses_its_role_with_its_owner() {
    let parts = [
        "<thead id=t><tr><td>x",
        "<tbody id=t aria-label=B><tr><td>x",
        "<tfoot id=t><tr><td>x",
        "<tr id=t><td>x",
        "<tr><td id=t>x",
        "<tr><th id=t scope=row>x",
    ];
    for owner in ["presentation", "none", "group", "list"] {
        for part in parts {
            let html = format!("<table role={owner}>{part}</table>");
            assert_eq!(r(&html), Some("generic"), "{html}");
        }
        let html = format!("<table role={owner}><tr id=t role=row><td>x</table>");
        assert_eq!(r(&html), Some("row"), "{html}");
    }
    for (owner, cell) in [("table", "cell"), ("grid", "gridcell")] {
        let html = format!("<table role={owner}><tr><td id=t>x</table>");
        assert_eq!(r(&html), Some(cell), "{html}");
    }
    let nested = "<table role=none><tr><td><table><tr id=t><th>h</table></table>";
    assert_eq!(r(nested), Some("row"));
    for owner in ["none", "presentation", "group", "menu"] {
        let html = format!("<ul role={owner}><li id=t>x</ul>");
        assert_eq!(r(&html), Some("generic"), "{html}");
    }
    for owner in ["list", "directory"] {
        let html = format!("<ol role={owner}><li id=t>x</ol>");
        assert_eq!(r(&html), Some("listitem"), "{html}");
    }
    let stands = "<ul role=none><li id=t role=listitem>x</ul>";
    assert_eq!(r(stands), Some("listitem"));
}

/// Chrome 158 (`bl-fc83`): a DPUB-ARIA role stands on any element, over the
/// element's own role.
#[test]
fn a_dpub_role_is_an_author_role() {
    assert_eq!(
        r("<a id=t role=doc-noteref href=#n>1</a>"),
        Some("doc-noteref")
    );
    assert_eq!(
        r("<aside id=t role=doc-footnote>n</aside>"),
        Some("doc-footnote")
    );
    assert_eq!(r("<li id=t role=doc-endnote>n</li>"), Some("doc-endnote"));
    assert_eq!(
        r("<div id=t role='doc-bogus doc-tip'>n</div>"),
        Some("doc-tip")
    );
}
