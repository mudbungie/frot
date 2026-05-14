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
    assert_eq!(role(&el_attr("div", &[("role", "wibble")])), Some("generic"));
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
    assert_eq!(role(&el_attr("input", &[("type", "checkbox")])), Some("checkbox"));
    assert_eq!(role(&el_attr("input", &[("type", "radio")])), Some("radio"));
}

#[test]
fn input_range_is_slider() {
    assert_eq!(role(&el_attr("input", &[("type", "range")])), Some("slider"));
}

#[test]
fn input_search_is_searchbox() {
    assert_eq!(role(&el_attr("input", &[("type", "search")])), Some("searchbox"));
}

#[test]
fn input_textlike_types_are_textbox() {
    for t in ["email", "tel", "url", "password", "number"] {
        assert_eq!(role(&el_attr("input", &[("type", t)])), Some("textbox"));
    }
}

#[test]
fn input_unknown_type_is_textbox() {
    assert_eq!(role(&el_attr("input", &[("type", "color")])), Some("textbox"));
}

#[test]
fn input_uppercase_type_is_normalized() {
    assert_eq!(role(&el_attr("input", &[("type", "CHECKBOX")])), Some("checkbox"));
}

#[test]
fn select_default_is_combobox() {
    assert_eq!(role(&el("select")), Some("combobox"));
}

#[test]
fn select_with_multiple_is_listbox() {
    assert_eq!(role(&el_attr("select", &[("multiple", "")])), Some("listbox"));
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
    assert_eq!(role(&el_attr("select", &[("size", "xx")])), Some("combobox"));
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
    for (tag, lvl) in [("h1", 1u32), ("h2", 2), ("h3", 3), ("h4", 4), ("h5", 5), ("h6", 6)] {
        assert_eq!(role(&el(tag)), Some("heading"));
        assert_eq!(level(&el(tag)), Some(lvl));
    }
}

#[test]
fn aria_level_overrides_heading_level() {
    let mut e = el("h2");
    e.attrs.push(Attr {
        name: "aria-level".into(),
        value: "5".into(),
    });
    assert_eq!(level(&e), Some(5));
}

#[test]
fn aria_level_zero_or_negative_ignored() {
    let mut e = el("h2");
    e.attrs.push(Attr {
        name: "aria-level".into(),
        value: "0".into(),
    });
    assert_eq!(level(&e), Some(2));
}

#[test]
fn aria_level_unparseable_ignored() {
    let mut e = el("h2");
    e.attrs.push(Attr {
        name: "aria-level".into(),
        value: "high".into(),
    });
    assert_eq!(level(&e), Some(2));
}

#[test]
fn non_heading_has_no_level() {
    assert_eq!(level(&el("div")), None);
}

#[test]
fn semantic_landmarks_mapped() {
    assert_eq!(role(&el("main")), Some("main"));
    assert_eq!(role(&el("nav")), Some("navigation"));
    assert_eq!(role(&el("aside")), Some("complementary"));
    assert_eq!(role(&el("header")), Some("banner"));
    assert_eq!(role(&el("footer")), Some("contentinfo"));
    assert_eq!(role(&el("section")), Some("region"));
    assert_eq!(role(&el("article")), Some("article"));
}

#[test]
fn lists_and_listitems() {
    assert_eq!(role(&el("ul")), Some("list"));
    assert_eq!(role(&el("ol")), Some("list"));
    assert_eq!(role(&el("dl")), Some("list"));
    assert_eq!(role(&el("menu")), Some("list"));
    assert_eq!(role(&el("li")), Some("listitem"));
}

#[test]
fn table_subroles() {
    assert_eq!(role(&el("table")), Some("table"));
    assert_eq!(role(&el("tr")), Some("row"));
    assert_eq!(role(&el("td")), Some("cell"));
    assert_eq!(role(&el("th")), Some("columnheader"));
    assert_eq!(role(&el("thead")), Some("rowgroup"));
    assert_eq!(role(&el("tbody")), Some("rowgroup"));
    assert_eq!(role(&el("tfoot")), Some("rowgroup"));
}

#[test]
fn br_and_meta_have_no_role() {
    assert_eq!(role(&el("br")), None);
    assert_eq!(role(&el("meta")), None);
    assert_eq!(role(&el("link")), None);
    assert_eq!(role(&el("title")), None);
    assert_eq!(role(&el("script")), None);
    assert_eq!(role(&el("style")), None);
    assert_eq!(role(&el("head")), None);
    assert_eq!(role(&el("html")), None);
}

#[test]
fn inline_text_roles() {
    assert_eq!(role(&el("strong")), Some("strong"));
    assert_eq!(role(&el("em")), Some("emphasis"));
    assert_eq!(role(&el("code")), Some("code"));
    assert_eq!(role(&el("mark")), Some("mark"));
    assert_eq!(role(&el("sub")), Some("subscript"));
    assert_eq!(role(&el("sup")), Some("superscript"));
    assert_eq!(role(&el("p")), Some("paragraph"));
    assert_eq!(role(&el("ins")), Some("insertion"));
    assert_eq!(role(&el("del")), Some("deletion"));
    assert_eq!(role(&el("s")), Some("deletion"));
    assert_eq!(role(&el("dfn")), Some("term"));
    assert_eq!(role(&el("dt")), Some("term"));
    assert_eq!(role(&el("dd")), Some("definition"));
    assert_eq!(role(&el("dialog")), Some("dialog"));
    assert_eq!(role(&el("blockquote")), Some("blockquote"));
}

#[test]
fn misc_roles() {
    assert_eq!(role(&el("hr")), Some("separator"));
    assert_eq!(role(&el("textarea")), Some("textbox"));
    assert_eq!(role(&el("progress")), Some("progressbar"));
    assert_eq!(role(&el("output")), Some("status"));
    assert_eq!(role(&el("meter")), Some("meter"));
    assert_eq!(role(&el("form")), Some("form"));
    assert_eq!(role(&el("fieldset")), Some("group"));
    assert_eq!(role(&el("figure")), Some("figure"));
    assert_eq!(role(&el("figcaption")), Some("caption"));
    assert_eq!(role(&el("caption")), Some("caption"));
    assert_eq!(role(&el("details")), Some("group"));
    assert_eq!(role(&el("optgroup")), Some("group"));
    assert_eq!(role(&el("option")), Some("option"));
    assert_eq!(role(&el("datalist")), Some("listbox"));
    assert_eq!(role(&el("summary")), Some("button"));
    assert_eq!(role(&el("time")), Some("time"));
    assert_eq!(role(&el("math")), Some("math"));
    assert_eq!(role(&el("svg")), Some("graphics-document"));
    assert_eq!(role(&el("body")), Some("generic"));
    assert_eq!(role(&el("span")), Some("generic"));
}
