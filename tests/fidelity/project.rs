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

/// A resolved URL in terms of the corpus it was read from: relative to the
/// corpus root, else to the root's origin, else as written. Chrome read the
/// fixture at the capture's neutral origin and frot at a `file://` path; the
/// *reference* each resolved is the same, and that is what is compared. The
/// root, not the page's own directory: a page in `saved/` may point above
/// itself (`../index.html`) and still be inside the corpus.
pub fn reference(href: &str, root: &str) -> String {
    let host = root.find("://").expect("a corpus URL has a scheme") + 3;
    let origin = &root[..host + root[host..].find('/').expect("a corpus URL has a path")];
    let rest = href
        .strip_prefix(root)
        .or_else(|| href.strip_prefix(origin));
    rest.unwrap_or(href).to_string()
}

/// `links`, Chrome's side: every `:any-link` element, as reference +
/// collapsed text.
pub fn chrome_links(links: &Value, root: &str) -> Vec<String> {
    let line = |l: &Value| link_line(text_of(&l["href"]), text_of(&l["text"]), root);
    items(links).iter().map(line).collect()
}

/// `links`, frot's side: the anchor kinds only. The view also carries
/// `<link>`, which is a relation and no hyperlink.
pub fn frot_links(out: &Value, root: &str) -> Vec<String> {
    let anchors = items(out).iter().filter(|l| l["kind"] != "link");
    let line = |l: &Value| link_line(text_of(&l["href"]), text_of(&l["text"]), root);
    anchors.map(line).collect()
}

fn link_line(href: &str, text: &str, root: &str) -> String {
    format!("{} {:?}", reference(href, root), collapse(text))
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
pub fn chrome_forms(forms: &Value, root: &str) -> Vec<String> {
    let mut out = Vec::new();
    for f in items(forms) {
        let action = reference(text_of(&f["action"]), root);
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
pub fn frot_forms(out: &Value, root: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for f in items(out) {
        let action = reference(text_of(&f["action"]), root);
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
fn a_reference_is_relative_to_the_corpus_it_was_read_from() {
    let file = "file:///work/fixtures/";
    let http = "http://fidelity.invalid/corpus/";
    for root in [file, http] {
        assert_eq!(reference(&format!("{root}other.html"), root), "other.html");
        assert_eq!(
            reference(&format!("{root}saved/page.html#frag"), root),
            "saved/page.html#frag"
        );
        assert_eq!(
            reference("https://example.com/far", root),
            "https://example.com/far"
        );
    }
    assert_eq!(reference("file:///rooted/x", file), "/rooted/x");
    assert_eq!(
        reference("http://fidelity.invalid/rooted/x", http),
        "/rooted/x"
    );
}
