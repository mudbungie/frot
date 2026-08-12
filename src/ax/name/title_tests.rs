//! `title` as a name source, and *where* — accname §2I is positional.
//!
//! Every case is a Chrome 139 measurement (`Accessibility.getFullAXTree`,
//! `--headless=new`), taken in `bl-d8ff`. The rule they spell out:
//! `title` speaks for a node that produced nothing, at the node a computation
//! is *about* and everywhere inside an `aria-labelledby` reference — never at a
//! descendant read only for its contribution.
//!
//! Reference probes are `<div role=button aria-labelledby=t>fb</div>`, so a
//! reference that resolved to nothing shows as "fb" rather than passing quietly.

use super::*;

fn name(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), None)
}

/// The defect: a reference target's `title` was never read, at the target or
/// anywhere under it.
#[test]
fn a_reference_reads_title_at_every_node() {
    for (html, want) in [
        // Chrome: "TIT" — the target itself, with nothing else to say.
        ("<span id=t title=TIT></span>", "TIT"),
        // Chrome: "TIT" — whitespace-only contents are still nothing said.
        ("<span id=t title=TIT>   </span>", "TIT"),
        // Chrome: "TIT" — a child of the target, one level down …
        ("<span id=t><span title=TIT></span></span>", "TIT"),
        // … and two, so it is the traversal and not the target's own step.
        (
            "<span id=t><span><span title=TIT></span></span></span>",
            "TIT",
        ),
        // Chrome: "TIT" — `alt=""` *is* an alternative, and an empty one still
        // leaves the node having said nothing.
        ("<img id=t alt='' title=TIT>", "TIT"),
        // Chrome: "TIT" — §2B does not recurse, so the target's own reference
        // is not followed and its `title` is what is left.
        (
            "<span id=t aria-labelledby=u title=TIT></span><span id=u>U</span>",
            "TIT",
        ),
    ] {
        assert_eq!(
            name(
                &format!("<div role=button aria-labelledby=t>fb</div>{html}"),
                "div"
            ),
            Some(want.into()),
            "{html}"
        );
    }
}

/// §2I is *last*: anything the node actually said wins, at the target and below.
#[test]
fn title_loses_to_anything_the_node_said() {
    for (html, want) in [
        // Chrome: "TXT" — contents.
        ("<span id=t title=TIT>TXT</span>", "TXT"),
        // Chrome: "AL" — `aria-label`.
        ("<span id=t title=TIT aria-label=AL></span>", "AL"),
        // Chrome: "A" — a non-empty native alternative.
        ("<img id=t alt=A title=TIT>", "A"),
        // Chrome: "X" — a descendant's own contents.
        ("<span id=t><span title=TIT>X</span></span>", "X"),
        // Chrome: "A TIT B" — the silent child speaks between its siblings,
        // fenced like any other alternative.
        ("<span id=t>A<span title=TIT></span>B</span>", "A TIT B"),
    ] {
        assert_eq!(
            name(
                &format!("<div role=button aria-labelledby=t>fb</div>{html}"),
                "div"
            ),
            Some(want.into()),
            "{html}"
        );
    }
}

/// The negative that makes this a rule about position rather than a row in the
/// alternative table: a plain name-from-contents recursion never reads `title`.
#[test]
fn a_contents_recursion_is_deaf_to_title() {
    // Chrome: "AB" — not "A TT B".
    assert_eq!(
        name("<div role=button>A<span title=TT></span>B</div>", "div"),
        Some("AB".into())
    );
    // Chrome: "" (nameless) — the child's `title` does not rescue an otherwise
    // empty button either.
    assert_eq!(
        name("<div role=button><span title=TT></span></div>", "div"),
        None
    );
    // Chrome: "X" — and it does not join contents that did appear.
    assert_eq!(
        name("<div role=button><span title=TT>X</span></div>", "div"),
        Some("X".into())
    );
}

/// The element being named reads its own `title` — step 4 of
/// [`accessible_name`], unchanged and still last.
#[test]
fn the_named_element_reads_its_own_title() {
    // Chrome: "TT".
    assert_eq!(
        name("<div role=button title=TT></div>", "div"),
        Some("TT".into())
    );
    // Chrome: "C" — contents win.
    assert_eq!(
        name("<div role=button title=TT>C</div>", "div"),
        Some("C".into())
    );
}

/// A `<label>` and a `<legend>` stand where the named element stands: their own
/// `title` counts, and their descendants' does not.
#[test]
fn a_label_or_legend_reads_only_its_own_title() {
    for (html, tag, want) in [
        // Chrome: "TL" — `for=`, wrapping, and a legend, each with nothing else.
        (
            "<label for=q title=TL></label><input id=q>",
            "input",
            Some("TL"),
        ),
        ("<label title=TL><input id=q></label>", "input", Some("TL")),
        (
            "<fieldset><legend title=TL></legend><input></fieldset>",
            "fieldset",
            Some("TL"),
        ),
        // Chrome: "LT" — its own text wins over its own `title`.
        (
            "<label for=q title=TL>LT</label><input id=q>",
            "input",
            Some("LT"),
        ),
        (
            "<fieldset><legend title=TL>LT</legend><input></fieldset>",
            "fieldset",
            Some("LT"),
        ),
        // Chrome: nameless — a child's `title` inside a label or legend is not
        // read, exactly as in any other contents recursion.
        (
            "<label for=q><span title=TL></span></label><input id=q>",
            "input",
            None,
        ),
        (
            "<fieldset><legend><span title=TL></span></legend><input></fieldset>",
            "fieldset",
            None,
        ),
        // Chrome: "AB" — and it does not appear between the label's own words.
        (
            "<label for=q>A<span title=TL></span>B</label><input id=q>",
            "input",
            Some("AB"),
        ),
        (
            "<fieldset><legend>A<span title=TL></span>B</legend><input></fieldset>",
            "fieldset",
            Some("AB"),
        ),
    ] {
        assert_eq!(name(html, tag), want.map(str::to_string), "{html}");
    }
}

/// `title` sits below every other source (`bl-4093`'s ordering, re-measured):
/// `aria-label` > `aria-labelledby` > the native name > `title`.
#[test]
fn title_stays_the_last_source() {
    for (html, tag, want) in [
        // Chrome: "TT" — an unlabelled `<optgroup>` is named by its `title`.
        (
            "<select><optgroup title=TT><option>a</option></optgroup></select>",
            "optgroup",
            "TT",
        ),
        // Chrome: "LB" — the `label` content attribute beats it.
        (
            "<select><optgroup label=LB title=TT><option>a</option></optgroup></select>",
            "optgroup",
            "LB",
        ),
        // Chrome: "AL" — and `aria-label` beats both.
        (
            "<select><optgroup aria-label=AL label=LB><option>a</option></optgroup></select>",
            "optgroup",
            "AL",
        ),
        // Chrome: "AL" / "LT" — on a control, `aria-label` and a `<label>` each
        // beat the control's own `title`.
        ("<input aria-label=AL title=TT>", "input", "AL"),
        (
            "<label for=q>LT</label><input id=q title=TT>",
            "input",
            "LT",
        ),
    ] {
        assert_eq!(name(html, tag), Some(want.into()), "{html}");
    }
}

/// The hidden rules are untouched: only the *directly* referenced node is read
/// while hidden, so a hidden child's `title` is dropped with the child.
#[test]
fn hidden_still_decides_before_title() {
    // Chrome: "TIT" — the target itself, hidden, still speaks its `title`.
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div>\
             <span id=t aria-hidden=true title=TIT></span>",
            "div"
        ),
        Some("TIT".into())
    );
    // Chrome: "fb" — a hidden *child* of the target contributes nothing, and
    // the reference resolves to nothing.
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div>\
             <span id=t><span aria-hidden=true title=TIT></span></span>",
            "div"
        ),
        Some("fb".into())
    );
    // Chrome: "fb" — a target with neither contents nor `title` still resolves
    // to nothing rather than to the empty string.
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div><span id=t></span>",
            "div"
        ),
        Some("fb".into())
    );
}
