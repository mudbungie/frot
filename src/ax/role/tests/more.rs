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
