//! A text field's placeholder as its name, and what an empty field says inside
//! someone else's name. Every case is Chrome 158 (`Accessibility.getFullAXTree`,
//! `bl-2d76`), held in `tests/fixtures/fidelity/names.html`.

use super::*;

fn name_of(html: &str, id: &str) -> Option<String> {
    let d = Document::parse(html);
    accessible_name(&d, d.element_by_id(id).unwrap(), None)
}

#[test]
fn a_placeholder_names_a_field_after_label_and_title() {
    for (html, want) in [
        ("<input id=x placeholder=P>", Some("P")),
        ("<input id=x type=number placeholder='P  Q'>", Some("P Q")),
        ("<textarea id=x placeholder=P></textarea>", Some("P")),
        ("<input id=x placeholder=P title=T>", Some("T")),
        ("<label>L <input id=x placeholder=P></label>", Some("L")),
        ("<input id=x placeholder=P aria-placeholder=A>", Some("P")),
        ("<input id=x aria-placeholder=A>", Some("A")),
        (
            "<span id=x role=searchbox aria-placeholder=A></span>",
            Some("A"),
        ),
        ("<input id=x placeholder='  '>", None),
        ("<input id=x type=checkbox placeholder=P>", None),
        ("<input id=x type=date placeholder=P>", None),
        ("<div id=x role=textbox placeholder=P></div>", None),
        ("<span id=x role=combobox aria-placeholder=A></span>", None),
    ] {
        assert_eq!(name_of(html, "x").as_deref(), want, "{html}");
    }
}

#[test]
fn an_empty_field_says_its_own_name_inside_another() {
    for (html, want) in [
        ("<h3 id=h>a<input placeholder=P>c</h3>", "a P c"),
        ("<h3 id=h>a<input placeholder=P value=V>c</h3>", "a V c"),
        ("<h3 id=h>a<input title=T>c</h3>", "a T c"),
        (
            "<h3 id=h>a<textarea placeholder=P></textarea>c</h3>",
            "a P c",
        ),
        (
            "<h3 id=h>a<textarea placeholder=P>t</textarea>c</h3>",
            "a t c",
        ),
        (
            "<h3 id=h>a<input id=i>c</h3><label for=i>L</label>",
            "a L c",
        ),
        (
            "<h3 id=h>a<input id=i title=T>c</h3><label for=i> </label>",
            "a T c",
        ),
        ("<h3 id=h>a<input>c</h3>", "a c"),
        ("<h3 id=h>a<input type=button>c</h3>", "a c"),
        (
            "<h3 id=h>a<span role=textbox aria-placeholder=A></span>c</h3>",
            "a c",
        ),
    ] {
        assert_eq!(name_of(html, "h").as_deref(), Some(want), "{html}");
    }
}

/// Two empty fields, each inside the other's label: reading one label reads
/// the other field, whose label holds the first — spent, so the walk ends.
#[test]
fn fields_labelled_by_each_others_labels_end() {
    let html = "<label for=a>A<input id=b></label><label for=b>B<input id=a></label>";
    assert_eq!(name_of(html, "a").as_deref(), Some("A B"));
}
