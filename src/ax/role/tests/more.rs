//! Overflow tests split from the parent module to hold the 300-line cap.
use super::*;

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
