//! The projections of the flat channels (`fidelity.md` §2.2, §3.1): each maps
//! one side's raw answer onto frot's declared model as a sequence of lines, so
//! the predicate over them can be plain equality. Chrome's side reads the
//! capture; frot's side reads the `out` of the matching view.
//!
//! What a projection leaves out is the acceptance policy — formatting,
//! coordinates, the capture's serving origin — and it is left out of BOTH
//! sides by construction, never tolerated after the fact.

use serde_json::Value;

fn items(v: &Value) -> &[Value] {
    v.as_array().expect("an array")
}

fn text_of(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `text`: formatting is out, the token sequence is in.
pub fn tokens(text: &Value) -> Vec<String> {
    text_of(text)
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// A resolved URL in terms of the page it sits on: relative to the document's
/// directory, else to its origin, else as written. Chrome read the fixture at
/// the capture's neutral origin and frot at a `file://` path; the *reference*
/// each resolved is the same, and that is what is compared.
pub fn reference(href: &str, page: &str) -> String {
    let dir = &page[..=page.rfind('/').expect("a page URL has a path")];
    let host = page.find("://").expect("a page URL has a scheme") + 3;
    let origin = &page[..host + page[host..].find('/').expect("a page URL has a path")];
    let rest = href.strip_prefix(dir).or_else(|| href.strip_prefix(origin));
    rest.unwrap_or(href).to_string()
}

/// `links`, Chrome's side: `document.links`, as reference + collapsed text.
pub fn chrome_links(links: &Value, page: &str) -> Vec<String> {
    let line = |l: &Value| link_line(text_of(&l["href"]), text_of(&l["text"]), page);
    items(links).iter().map(line).collect()
}

/// `links`, frot's side: the anchor kinds only. The view also carries
/// `<link>`, which `document.links` has no notion of.
pub fn frot_links(out: &Value, page: &str) -> Vec<String> {
    let anchors = items(out).iter().filter(|l| l["kind"] != "link");
    let line = |l: &Value| link_line(text_of(&l["href"]), text_of(&l["text"]), page);
    anchors.map(line).collect()
}

fn link_line(href: &str, text: &str, page: &str) -> String {
    format!("{} {:?}", reference(href, page), collapse(text))
}

/// The controls frot's `forms` view declares (`src/views/forms.rs`).
const FIELDS: &[&str] = &["input", "textarea", "select", "button"];

fn field_line(tag: &str, kind: &Value, name: &Value) -> String {
    // The view reports `type` as written, and only where HTML gives it meaning.
    let kind = if matches!(tag, "input" | "button") {
        text_of(kind)
    } else {
        ""
    };
    format!("  {tag} type={kind:?} name={:?}", text_of(name))
}

/// `forms`, Chrome's side: `document.forms` and each form's `elements`,
/// narrowed to the four control tags.
pub fn chrome_forms(forms: &Value, page: &str) -> Vec<String> {
    let mut out = Vec::new();
    for f in items(forms) {
        let action = reference(text_of(&f["action"]), page);
        out.push(format!("form {action} {}", text_of(&f["method"])));
        for e in items(&f["elements"]) {
            let tag = text_of(&e["tag"]);
            if FIELDS.contains(&tag) {
                out.push(field_line(tag, &e["typeAttr"], &e["name"]));
            }
        }
    }
    out
}

/// `forms`, frot's side.
pub fn frot_forms(out: &Value, page: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for f in items(out) {
        let action = reference(text_of(&f["action"]), page);
        lines.push(format!("form {action} {}", text_of(&f["method"])));
        for e in items(&f["fields"]) {
            lines.push(field_line(text_of(&e["tag"]), &e["type"], &e["name"]));
        }
    }
    lines
}

/// `bboxes`, Chrome's side: box or no box only — the elements the recorder
/// counted a client rect for (it counts none where `checkVisibility()` is
/// false), by source-order index. No coordinate is read, ever.
pub fn chrome_boxes(boxes: &Value) -> Vec<String> {
    let painted = |(_, b): &(usize, &Value)| b["rects"].as_u64() > Some(0);
    let line = |(i, b): (usize, &Value)| format!("{i} {}", text_of(&b["tag"]));
    items(boxes)
        .iter()
        .enumerate()
        .filter(painted)
        .map(line)
        .collect()
}

/// `bboxes`, frot's side: the elements the view lists, put back in source
/// order (the view is in reading order). The rect is not read.
pub fn frot_boxes(out: &Value) -> Vec<String> {
    let mut boxes: Vec<(u64, &str)> = items(out)
        .iter()
        .map(|b| (b["i"].as_u64().expect("i"), text_of(&b["tag"])))
        .collect();
    boxes.sort();
    boxes.iter().map(|(i, tag)| format!("{i} {tag}")).collect()
}

#[test]
fn a_reference_is_relative_to_the_page_it_sits_on() {
    let file = "file:///work/fixtures/page.html";
    let http = "http://fidelity.invalid/corpus/page.html";
    for page in [file, http] {
        let dir = &page[..page.len() - "page.html".len()];
        assert_eq!(reference(&format!("{dir}other.html"), page), "other.html");
        assert_eq!(reference(&format!("{page}#frag"), page), "page.html#frag");
        assert_eq!(
            reference("https://example.com/far", page),
            "https://example.com/far"
        );
    }
    assert_eq!(reference("file:///rooted/x", file), "/rooted/x");
    assert_eq!(
        reference("http://fidelity.invalid/rooted/x", http),
        "/rooted/x"
    );
}
