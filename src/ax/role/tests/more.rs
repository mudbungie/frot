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
