//! `Document::unpainted` / `Document::concealed`: the UA rules that withhold a
//! node for what its *parent* is — a closed `<details>`'s disclosure content
//! (`bl-74a6`) and a fallback-content element's children (`bl-0f83`,
//! `bl-e79a`). The two queries differ on `<canvas>` alone, which paints no
//! child and hides none from the accessibility tree. The attribute-local half
//! of the same question is `Element::hidden` (`bl-eeb4`).

use super::*;

fn first_element(doc: &Document, name: &str) -> NodeId {
    *doc.find_by_tag(name).first().unwrap()
}

/// `(label, concealed)` for each child of the first `<details>` in `html` —
/// element children labelled by tag, text children by their content.
fn concealment(html: &str) -> Vec<(String, bool)> {
    let doc = Document::parse(html);
    let d = first_element(&doc, "details");
    doc.node(d)
        .children
        .iter()
        .map(|&c| {
            let label = match &doc.node(c).kind {
                NodeKind::Element(el) => el.name.clone(),
                kind => format!("{kind:?}"),
            };
            (label, doc.concealed(c))
        })
        .collect()
}

#[test]
fn a_closed_details_conceals_everything_but_its_first_summary() {
    assert_eq!(
        concealment("<details><summary>S</summary><p>body</p>raw</details>"),
        [
            ("summary".into(), false),
            ("p".into(), true),
            (r#"Text("raw")"#.into(), true),
        ]
    );
}

#[test]
fn an_open_details_conceals_nothing() {
    // `open` is a boolean attribute: presence is the fact, the value is noise.
    for html in [
        "<details open><summary>S</summary><p>body</p></details>",
        "<details open='false'><summary>S</summary><p>body</p></details>",
    ] {
        assert_eq!(
            concealment(html),
            [("summary".into(), false), ("p".into(), false)],
            "{html}"
        );
    }
}

#[test]
fn only_the_first_summary_is_the_disclosure_control() {
    // A later `<summary>` is disclosure content like any other child
    // (`details > summary:first-of-type`).
    assert_eq!(
        concealment("<details><summary>one</summary><summary>two</summary></details>"),
        [("summary".into(), false), ("summary".into(), true)]
    );
}

#[test]
fn a_closed_details_with_no_summary_conceals_every_child() {
    // The UA supplies its own disclosure control; nothing in the document is it.
    assert_eq!(
        concealment("<details><p>body</p></details>"),
        [("p".into(), true)]
    );
}

#[test]
fn nothing_outside_a_closed_details_is_concealed() {
    let doc = Document::parse("<details open><summary>S</summary></details><p>after</p>");
    // A root has no parent to conceal it, and a non-`details` parent conceals
    // nothing — including the `<summary>`'s own text child.
    for id in 0..doc.len() as NodeId {
        assert!(!doc.concealed(id), "{id}");
    }
}

/// `(label, concealed)` for each child of the first `<video>`/`<audio>` in
/// `html`, in the same shape [`concealment`] uses for `<details>`.
fn media_concealment(html: &str, tag: &str) -> Vec<(String, bool)> {
    let doc = Document::parse(html);
    let m = first_element(&doc, tag);
    doc.node(m)
        .children
        .iter()
        .map(|&c| {
            let label = match &doc.node(c).kind {
                NodeKind::Element(el) => el.name.clone(),
                kind => format!("{kind:?}"),
            };
            (label, doc.concealed(c))
        })
        .collect()
}

#[test]
fn a_media_element_conceals_every_child() {
    // `<video>`/`<audio>` descendants are fallback for a UA without the
    // element; frot has it, so nothing inside is rendered — `<source>` and
    // `<track>` least of all.
    for tag in ["video", "audio"] {
        let html = format!(
            "<{tag} controls poster=p.jpg><source src=a.mp4><track src=c.vtt>\
             <p>Sorry, your browser does not support embedded videos</p>raw</{tag}>"
        );
        assert_eq!(
            media_concealment(&html, tag),
            [
                ("source".into(), true),
                ("track".into(), true),
                ("p".into(), true),
                (r#"Text("raw")"#.into(), true),
            ],
            "{tag}"
        );
    }
}

#[test]
fn a_media_element_with_no_source_still_conceals_its_fallback() {
    // Fallback is not an error state: a missing or broken source yields an
    // empty player, never the prose.
    assert_eq!(
        media_concealment("<video><p>no video</p></video>", "video"),
        [("p".into(), true)]
    );
}

/// `(label, unpainted, concealed)` for each child of the first `<tag>` in
/// `html` — the two structural queries side by side, which is the whole point
/// of `bl-e79a`: for `<canvas>` they disagree.
fn structural(html: &str, tag: &str) -> Vec<(String, bool, bool)> {
    let doc = Document::parse(html);
    let m = first_element(&doc, tag);
    doc.node(m)
        .children
        .iter()
        .map(|&c| {
            let label = match &doc.node(c).kind {
                NodeKind::Element(el) => el.name.clone(),
                kind => format!("{kind:?}"),
            };
            (label, doc.unpainted(c), doc.concealed(c))
        })
        .collect()
}

#[test]
fn a_canvas_paints_no_child_and_conceals_none() {
    // Chrome 139: `<canvas>x</canvas>` gives `x` zero client rects and no
    // `innerText`, and still exposes `StaticText "x"` in the AX tree — HTML
    // makes canvas fallback the element's accessible sub-tree. `unpainted`
    // and `concealed` are what carry that split (`bl-e79a`).
    assert_eq!(
        structural("<canvas width=60><p>no canvas</p>raw</canvas>", "canvas"),
        [
            ("p".into(), true, false),
            (r#"Text("raw")"#.into(), true, false),
        ]
    );
}

#[test]
fn a_replaced_subtree_element_both_unpaints_and_conceals() {
    // The rest of the fallback set has one answer for both questions: the UA
    // replaces the subtree outright, so neither the page nor the AX tree has
    // it. `<iframe>` joins the media pair on the same measurement.
    for tag in ["video", "audio"] {
        let html = format!("<{tag}><p>fallback</p></{tag}>");
        assert_eq!(structural(&html, tag), [("p".into(), true, true)], "{tag}");
    }
    // An `<iframe>`'s content is parsed as raw text, never elements — Chrome
    // reports the same single text node — so its one child is the whole
    // markup, withheld all the same.
    assert_eq!(
        structural("<iframe><p>fallback</p></iframe>", "iframe"),
        [(r#"Text("<p>fallback</p>")"#.into(), true, true)]
    );
}

#[test]
fn object_picture_and_ordinary_parents_withhold_nothing() {
    // `<object>` fallback turns on a load outcome frot never learns, and a
    // `<picture>`'s children are not fallback at all — the `<img>` is the
    // rendered element (`bl-e79a`, measured, deliberately unchanged).
    for tag in ["object", "picture", "div", "span"] {
        let html = format!("<{tag}><p>inside</p>raw</{tag}>");
        assert_eq!(
            structural(&html, tag),
            [
                ("p".into(), false, false),
                (r#"Text("raw")"#.into(), false, false),
            ],
            "{tag}"
        );
    }
}

#[test]
fn a_closed_details_is_both_unpainted_and_concealed() {
    // The `<details>` rule is document *state*, not a tag fact, so it holds
    // for both queries and keeps the disclosure body out of the AX tree.
    assert_eq!(
        structural(
            "<details><summary>S</summary><p>body</p></details>",
            "details"
        ),
        [("summary".into(), false, false), ("p".into(), true, true)]
    );
}

/// The tags of `id`'s AX children — the accessor both `ax` walks descend
/// through, so this is exactly what `--out ax` can see and name.
fn ax_child_tags(doc: &Document, id: NodeId) -> Vec<String> {
    doc.ax_children(id)
        .map(|c| match &doc.node(c).kind {
            NodeKind::Element(el) => el.name.clone(),
            kind => format!("{kind:?}"),
        })
        .collect()
}

/// The tags of `id`'s painted children — what `views::text` can see.
fn painted_child_tags(doc: &Document, id: NodeId) -> Vec<String> {
    doc.painted_children(id)
        .map(|c| match &doc.node(c).kind {
            NodeKind::Element(el) => el.name.clone(),
            kind => format!("{kind:?}"),
        })
        .collect()
}

#[test]
fn the_two_accessors_split_exactly_where_their_predicates_do() {
    // `<canvas>` is the whole reason there are two of each (`bl-e79a`): the
    // page loses its fallback and the AX tree keeps it. A closed `<details>`
    // and a `<video>` are withheld from both.
    let doc = Document::parse(
        "<canvas><b>c</b></canvas><video><b>v</b></video>\
         <details><summary>s</summary><p>d</p></details>",
    );
    for tag in ["canvas", "video", "details"] {
        let id = first_element(&doc, tag);
        let want: &[&str] = if tag == "details" { &["summary"] } else { &[] };
        assert_eq!(painted_child_tags(&doc, id), want, "painted {tag}");
    }
    assert_eq!(ax_child_tags(&doc, first_element(&doc, "canvas")), ["b"]);
}

#[test]
fn only_the_replaced_subtree_elements_hide_their_children_from_ax() {
    let doc = Document::parse(
        "<video><b>f</b></video><audio><b>f</b></audio><iframe></iframe>\
         <canvas><b>f</b></canvas><object><b>f</b></object>\
         <picture><b>f</b></picture><div><b>f</b></div>",
    );
    let keeps: Vec<&str> = [
        "video", "audio", "iframe", "canvas", "object", "picture", "div",
    ]
    .into_iter()
    .filter(|t| !ax_child_tags(&doc, first_element(&doc, t)).is_empty())
    .collect();
    assert_eq!(keeps, ["canvas", "object", "picture", "div"]);
}

#[test]
fn a_closed_details_offers_the_ax_walks_its_summary_alone() {
    let doc = Document::parse("<details><summary>S</summary><p>body</p>raw</details>");
    assert_eq!(
        ax_child_tags(&doc, first_element(&doc, "details")),
        ["summary"]
    );
}

#[test]
fn a_parent_with_no_tag_hides_nothing() {
    // `insert_child` is deliberately permissive (`dom::mutate`), so a text node
    // can end up a parent. It has no tag to withhold by, and it is no closed
    // `<details>`, so the child is neither unpainted nor concealed.
    let mut doc = Document::parse("<p>t</p>");
    let text = doc.node(first_element(&doc, "p")).children[0];
    let child = doc.create_element("b");
    doc.insert_child(text, child, None);
    assert!(!doc.unpainted(child) && !doc.concealed(child));
    assert_eq!(ax_child_tags(&doc, text), ["b"]);
}
