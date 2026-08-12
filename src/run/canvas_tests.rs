//! `<object>`/`<canvas>`/`<picture>` fallback content end-to-end (`bl-e79a`),
//! the audit `bl-0f83` deferred rather than extending its media rule by
//! analogy.
//!
//! Oracle: Chrome 139 headless (`--headless=new`) at 1280×720, reading
//! `body.innerText`, `Range.getClientRects()` per text node, and
//! `Accessibility.getFullAXTree`. Measured 2026-08-11:
//!
//! | markup | painted | in AX |
//! |---|---|---|
//! | `<canvas>x</canvas>` | no (zero rects, absent from `innerText`) | **yes** (`StaticText "x"`) |
//! | `<iframe>x</iframe>` | no | no |
//! | `<object data=ok.png>x</object>` (loads) | no | no |
//! | `<object data=404.png>x</object>` | **yes** | **yes** |
//! | `<object data=ok.png type=unsupported>x</object>` | **yes** | **yes** |
//! | `<object>x</object>` (no `data`, no `type`) | **yes** | **yes** |
//! | `<object type=image/png>x</object>` (no `data`) | no | no |
//! | `<picture>x<img alt=a></picture>` | **yes** | **yes** (`StaticText`, plus `image "a"`) |
//!
//! Three different answers, so three different outcomes here:
//!
//! - **`<canvas>`** is the case a single boolean could not express. Its
//!   fallback content *is* its accessible sub-tree, so the page must lose it
//!   and the AX tree must keep it — `Document::unpainted` versus
//!   `Document::concealed`.
//! - **`<object>`** turns on a fetch result, a sniffed MIME type and plugin
//!   support. frot never fetches `<object data>`, so it cannot know which of
//!   the six rows above it is looking at, and guessing one would be inventing
//!   state. Deliberately unchanged: the fallback stays rendered, which is the
//!   copy the document actually carries.
//! - **`<picture>`** is not fallback at all — its `<source>`s are void
//!   configuration and its `<img>` is the rendered element. Deliberately
//!   unchanged.

use super::fallback_tests::{ax_roles, bbox_tags, serve_and_run};

/// A canvas carrying the two things authors put in one: prose for a UA without
/// it, and a real control that a screen-reader user is meant to reach.
const CANVAS: &str = "<html><body><h1>chart</h1>\
     <canvas width=300 height=150>\
     <p>Your browser does not support canvas.</p>\
     <button>Download the data</button>raw\
     </canvas><p>after</p></body></html>";

#[test]
fn canvas_fallback_is_not_page_text_in_either_recipe() {
    for args in [vec!["--out", "text"], vec!["--css", "--out", "text"]] {
        let v = serve_and_run(CANVAS, &args);
        assert_eq!(v["out"], "chart\nafter", "{args:?}");
    }
}

#[test]
fn canvas_fallback_stays_in_the_ax_tree_in_either_recipe() {
    // The half that must NOT follow the page: HTML defines canvas fallback as
    // the element's accessible sub-tree, and Chrome exposes every node of it.
    // Omitting text a screen-reader user does get is a VISION-5 violation in
    // the opposite direction from emitting text nobody sees.
    for args in [vec!["--out", "ax"], vec!["--css", "--out", "ax"]] {
        let roles = ax_roles(&serve_and_run(CANVAS, &args));
        assert_eq!(
            roles,
            vec!["heading", "paragraph", "button", "paragraph"],
            "{args:?}"
        );
    }
}

#[test]
fn canvas_fallback_still_names_the_element_around_it() {
    // The counterpart to `fallback_tests::fallback_prose_never_names_the
    // _element_around_it`: one accessor decides both, and it must let this
    // one through. Chrome 139 names `<a><canvas>CANVAS_FALLBACK</canvas></a>`
    // "CANVAS_FALLBACK" — the same markup with `<video>` names it from the
    // player, never the prose.
    let page = "<html><body><a href=/x><canvas width=100 height=50>\
         Chart of monthly totals</canvas></a></body></html>";
    for args in [vec!["--out", "ax"], vec!["--css", "--out", "ax"]] {
        let v = serve_and_run(page, &args);
        assert_eq!(v["out"][0]["role"], "link", "{args:?}");
        assert_eq!(v["out"][0]["name"], "Chart of monthly totals", "{args:?}");
    }
}

#[test]
fn canvas_fallback_gets_no_geometry() {
    // Chrome gives every canvas-fallback element a zero rect; `bboxes` is one
    // entry per *rendered* element, so it gets no entry at all — and the
    // canvas's own box is not sized from prose nothing paints.
    let v = serve_and_run(CANVAS, &["--css", "--out", "bboxes"]);
    let tags = bbox_tags(&v);
    assert!(tags.contains(&"canvas".into()), "{tags:?}");
    assert!(!tags.contains(&"button".into()), "{tags:?}");
    let canvas = v["out"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["tag"] == "canvas")
        .unwrap();
    assert_eq!(canvas["text"], serde_json::Value::Null);
    assert_eq!(canvas["rect"]["w"], 0);
}

#[test]
fn a_canvas_is_not_content_for_the_starvation_check() {
    // A page whose only body copy is canvas fallback paints nothing, so the
    // `text` view honestly has no impression — and a canvas has no content
    // until a script draws one, which makes `needs: js` literally right.
    let page = "<html><body><script src=/a.js></script>\
         <canvas>Your browser does not support canvas.</canvas></body></html>";
    let v = serve_and_run(page, &["--out", "text"]);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["js"]));
}

#[test]
fn authored_display_none_inside_canvas_fallback_still_hides_it() {
    // The `<canvas>` exception is confined to the structural rule: everything
    // else about the sub-tree is ordinary cascade, exactly as Chrome measured
    // (`<canvas><span style=display:none>x</span></canvas>` exposes no `x`).
    let page = "<html><body><canvas>\
         <button style='display:none'>hidden control</button>\
         <button>live control</button></canvas></body></html>";
    let roles = ax_roles(&serve_and_run(page, &["--css", "--out", "ax"]));
    assert_eq!(roles, vec!["button"]);
    assert_eq!(ax_roles(&serve_and_run(page, &["--out", "ax"])).len(), 2);
}

#[test]
fn object_fallback_is_left_rendered_because_the_load_outcome_is_unknown() {
    // Measured: Chrome shows it for a 404, an unsupported `type`, and a bare
    // `<object>`; hides it when the resource becomes the box. frot never
    // fetches `<object data>`, so it reports the copy the document carries
    // rather than inventing an outcome.
    let page = "<html><body><object data=/thing.pdf type=application/pdf>\
         <a href=/thing.pdf>Download the PDF</a></object></body></html>";
    assert_eq!(
        serve_and_run(page, &["--css", "--out", "text"])["out"],
        "Download the PDF"
    );
    assert_eq!(
        ax_roles(&serve_and_run(page, &["--css", "--out", "ax"])),
        vec!["link"]
    );
    assert!(bbox_tags(&serve_and_run(page, &["--css", "--out", "bboxes"])).contains(&"a".into()));
}

#[test]
fn picture_children_are_not_fallback() {
    // The `<img>` is the rendered element and a `<picture>`'s own text paints
    // like any inline text — measured, and already what frot did.
    let page = "<html><body><picture><source srcset=/a.webp type=image/webp>\
         caption<img src=/a.png alt='A chart'></picture></body></html>";
    assert_eq!(
        serve_and_run(page, &["--css", "--out", "text"])["out"],
        "caption"
    );
    assert_eq!(
        ax_roles(&serve_and_run(page, &["--css", "--out", "ax"])),
        vec!["img"]
    );
}

#[test]
fn iframe_fallback_is_withheld_everywhere() {
    // `<iframe>` content is parsed as raw text and a UA paints and exposes
    // none of it (measured). It moved out of `views::text`'s private skip set
    // into the shared fact, so geometry and the AX tree cut it at the same
    // place the text view does.
    let page = "<html><body><h1>page</h1><iframe src=/f.html>\
         Your browser does not support frames.</iframe></body></html>";
    assert_eq!(
        serve_and_run(page, &["--css", "--out", "text"])["out"],
        "page"
    );
    let v = serve_and_run(page, &["--css", "--out", "bboxes"]);
    let iframe = v["out"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["tag"] == "iframe")
        .unwrap();
    assert_eq!(iframe["text"], serde_json::Value::Null);
    assert_eq!(iframe["rect"]["w"], 0);
}
