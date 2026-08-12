//! `<optgroup label>` / `<option label>` — the HTML-AAM name from author for
//! the `<select>` subtree (`bl-4093`).
//!
//! Every case here was measured in Chrome 139 headless via
//! `Accessibility.getFullAXTree`, the same oracle `bl-66ed` used to map the
//! rest of the subtree.

use super::*;

fn name(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), None)
}

/// The ball's first reproduction: Chrome exposes the group as `group "GroupA"`
/// while the label text appears in no other channel — `innerText` drops it.
#[test]
fn optgroup_label_names_the_group() {
    let html = "<select><optgroup label='GroupA'><option>first</option></optgroup></select>";
    assert_eq!(name(html, "optgroup"), Some("GroupA".into()));
    assert_eq!(name(html, "option"), Some("first".into()));
}

/// The second reproduction: `<option label>` **replaces** the contents-derived
/// name (Chrome: option "L1"), while an option without one still names from
/// contents.
#[test]
fn option_label_replaces_the_contents_name() {
    let html = "<select><option label='L1'>text1</option><option>text2</option></select>";
    let d = Document::parse(html);
    let opts = d.find_by_tag("option");
    assert_eq!(accessible_name(&d, opts[0], None), Some("L1".into()));
    assert_eq!(accessible_name(&d, opts[1], None), Some("text2".into()));
}

/// An empty `label` is no label at all — unlike `alt=""`, which *is* the empty
/// alternative. Measured: `<option label="">text3</option>` is named "text3",
/// and an empty-labelled `<optgroup>` is nameless rather than named from its
/// options.
#[test]
fn empty_label_is_not_an_alternative() {
    assert_eq!(
        name("<select><option label=''>text3</option></select>", "option"),
        Some("text3".into())
    );
    let html = "<select><optgroup label=''><option>third</option></optgroup></select>";
    assert_eq!(name(html, "optgroup"), None);
}

/// A `group` is not a name-from-contents role, so an unlabelled `<optgroup>`
/// stays nameless instead of echoing its options (Chrome: name "").
#[test]
fn optgroup_without_a_label_has_no_name() {
    let html = "<select><optgroup><option>second</option></optgroup></select>";
    assert_eq!(name(html, "optgroup"), None);
}

/// Ordering against accname: `aria-label` outranks the content attribute on
/// both tags (Chrome: "A5", "AG4").
#[test]
fn aria_label_outranks_the_label_attribute() {
    assert_eq!(
        name(
            "<select><option label='L5' aria-label='A5'>text5</option></select>",
            "option"
        ),
        Some("A5".into())
    );
    let html =
        "<select><optgroup label='G4' aria-label='AG4'><option>x</option></optgroup></select>";
    assert_eq!(name(html, "optgroup"), Some("AG4".into()));
}

/// …and `title` is below it, as the last step of the algorithm: `label` wins
/// where both are present (Chrome: "G6"), `title` names an `<optgroup>` that
/// has no `label` (Chrome: "T5").
#[test]
fn title_is_below_the_label_attribute() {
    let g = |a: &str| format!("<select><optgroup {a}><option>x</option></optgroup></select>");
    assert_eq!(
        name(&g("label='G6' title='T6'"), "optgroup"),
        Some("G6".into())
    );
    assert_eq!(name(&g("title='T5'"), "optgroup"), Some("T5".into()));
}

/// The same fact seen from the §2F recursion: a descendant carrying the
/// attribute contributes it and is **not** descended into. Measured with a
/// wrapping `<label>`: `<label for=s>Pick <optgroup label=IG>ig</optgroup>
/// </label>` names the control "Pick IG", not "Pick IGig". An unlabelled group
/// has no alternative, so the recursion descends into it as usual.
#[test]
fn a_descendant_optgroup_contributes_its_label() {
    let l =
        |a: &str| format!("<label for='s'>Pick <optgroup {a}>ig</optgroup></label><input id='s'>");
    assert_eq!(name(&l("label='IG'"), "input"), Some("Pick IG".into()));
    assert_eq!(name(&l(""), "input"), Some("Pick ig".into()));
    let html = "<h2>Pick <option label='OL'>ot</option></h2>";
    assert_eq!(name(html, "h2"), Some("Pick OL".into()));
}

/// An embedded `<select>` speaks the selected option's *label*, not its text —
/// measured: Chrome names `<button>Choose <select><option label=X>y</option>
/// </select></button>` "Choose X", and "Choose y" when the label is empty.
#[test]
fn an_embedded_select_speaks_the_selected_options_label() {
    let b = |o: &str| format!("<button>Choose <select>{o}</select></button>");
    assert_eq!(
        name(&b("<option label='X'>y</option>"), "button"),
        Some("Choose X".into())
    );
    assert_eq!(
        name(&b("<option label=''>y</option>"), "button"),
        Some("Choose y".into())
    );
    assert_eq!(
        name(
            &b("<option>p</option><option label='Q' selected>q</option>"),
            "button"
        ),
        Some("Choose Q".into())
    );
}

/// The attribute is the tag's own fact, not a fact about `<select>`: a bare
/// `<optgroup>`/`<option>` in a `<div>` names the same way (Chrome: group
/// "BareG", option "BO"), which is how `<option>` joined `BLOCK_TAGS` too
/// (`layout.md` §2.4).
#[test]
fn the_label_names_outside_a_select_too() {
    let html = "<div><optgroup label='BareG'><option label='BO'>botext</option></optgroup></div>";
    assert_eq!(name(html, "optgroup"), Some("BareG".into()));
    assert_eq!(name(html, "option"), Some("BO".into()));
}
