use super::*;
use crate::dom::{Attr, Element};

fn el(name: &str) -> Element {
    Element {
        name: name.to_string(),
        attrs: vec![],
    }
}

fn el_attr(name: &str, attrs: &[(&str, &str)]) -> Element {
    Element {
        name: name.to_string(),
        attrs: attrs
            .iter()
            .map(|(n, v)| Attr {
                name: (*n).to_string(),
                value: (*v).to_string(),
            })
            .collect(),
    }
}

#[test]
fn anchor_with_href_is_link() {
    assert_eq!(role(&el_attr("a", &[("href", "/x")])), Some("link"));
}

#[test]
fn anchor_without_href_is_generic() {
    assert_eq!(role(&el("a")), Some("generic"));
}

#[test]
fn area_with_href_is_link() {
    assert_eq!(role(&el_attr("area", &[("href", "/x")])), Some("link"));
}

#[test]
fn explicit_role_wins_when_recognized() {
    assert_eq!(role(&el_attr("div", &[("role", "button")])), Some("button"));
}

#[test]
fn explicit_role_picks_first_token() {
    assert_eq!(
        role(&el_attr("div", &[("role", "navigation menu")])),
        Some("navigation")
    );
}

#[test]
fn explicit_unknown_role_falls_back_to_implicit() {
    assert_eq!(
        role(&el_attr("div", &[("role", "wibble")])),
        Some("generic")
    );
}

#[test]
fn empty_explicit_role_falls_back_to_implicit() {
    assert_eq!(role(&el_attr("div", &[("role", "")])), Some("generic"));
}

#[test]
fn buttons_and_button_inputs_are_buttons() {
    assert_eq!(role(&el("button")), Some("button"));
    for t in ["button", "image", "reset", "submit"] {
        assert_eq!(role(&el_attr("input", &[("type", t)])), Some("button"));
    }
}

#[test]
fn input_text_default_is_textbox() {
    assert_eq!(role(&el("input")), Some("textbox"));
}

#[test]
fn input_checkbox_and_radio() {
    assert_eq!(
        role(&el_attr("input", &[("type", "checkbox")])),
        Some("checkbox")
    );
    assert_eq!(role(&el_attr("input", &[("type", "radio")])), Some("radio"));
}

#[test]
fn input_range_is_slider() {
    assert_eq!(
        role(&el_attr("input", &[("type", "range")])),
        Some("slider")
    );
}

#[test]
fn input_search_is_searchbox() {
    assert_eq!(
        role(&el_attr("input", &[("type", "search")])),
        Some("searchbox")
    );
}

#[test]
fn input_textlike_types_are_textbox() {
    for t in ["email", "tel", "url", "password", "number"] {
        assert_eq!(role(&el_attr("input", &[("type", t)])), Some("textbox"));
    }
}

#[test]
fn input_unknown_type_is_textbox() {
    assert_eq!(
        role(&el_attr("input", &[("type", "color")])),
        Some("textbox")
    );
}

#[test]
fn input_uppercase_type_is_normalized() {
    assert_eq!(
        role(&el_attr("input", &[("type", "CHECKBOX")])),
        Some("checkbox")
    );
}

#[test]
fn select_default_is_combobox() {
    assert_eq!(role(&el("select")), Some("combobox"));
}

#[test]
fn select_with_multiple_is_listbox() {
    assert_eq!(
        role(&el_attr("select", &[("multiple", "")])),
        Some("listbox")
    );
}

#[test]
fn select_with_size_gt_1_is_listbox() {
    assert_eq!(role(&el_attr("select", &[("size", "5")])), Some("listbox"));
}

#[test]
fn select_with_size_1_is_combobox() {
    assert_eq!(role(&el_attr("select", &[("size", "1")])), Some("combobox"));
}

#[test]
fn select_with_unparseable_size_is_combobox() {
    assert_eq!(
        role(&el_attr("select", &[("size", "xx")])),
        Some("combobox")
    );
}

#[test]
fn img_with_alt_is_img() {
    assert_eq!(role(&el_attr("img", &[("alt", "logo")])), Some("img"));
}

#[test]
fn img_with_empty_alt_is_presentation() {
    assert_eq!(role(&el_attr("img", &[("alt", "")])), Some("presentation"));
}

#[test]
fn img_without_alt_is_img() {
    assert_eq!(role(&el("img")), Some("img"));
}

#[test]
fn headings_are_heading_with_levels() {
    for (tag, lvl) in [
        ("h1", 1u32),
        ("h2", 2),
        ("h3", 3),
        ("h4", 4),
        ("h5", 5),
        ("h6", 6),
    ] {
        assert_eq!(role(&el(tag)), Some("heading"));
        assert_eq!(level(&el(tag)), Some(lvl));
    }
}

mod more;
