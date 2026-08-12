//! Media fallback content end-to-end across every view that reports rendered
//! content (`bl-0f83`).
//!
//! Field repro 2026-08-11, `https://www.w3.org/` under `--css --out text`:
//! frot emitted "Sorry, your browser does not support embedded videos" and the
//! AX tree exposed it too. Chrome at 1280×720 paints the `<video>` as a
//! 500×281 replaced box and exposes neither — with JavaScript disabled as
//! well, because fallback is for a UA that does not *implement* `<video>`.
//! frot does, and presents a Firefox persona while doing it, so emitting the
//! fallback describes a different page.
//!
//! Unlike `[hidden]` (`bl-eeb4`) and a closed `<details>` (`bl-74a6`), this is
//! a content-model fact rather than a UA *stylesheet* one, so it holds in
//! every recipe: `--css` is not what makes it true, and the views that carry a
//! recipe-independent skip set already read it there
//! (`tags::renders_children`).

use super::*;

/// The w3.org shape: a media element with sources, a poster, a label, and
/// prose no browser shows.
const MEDIA: &str = "<html><body><p>before</p>\
     <video controls poster='p.jpg' aria-label='A demo'>\
     <source src='a.mp4' type='video/mp4'><track kind=captions src='c.vtt'>\
     <p>Sorry, your browser does not support embedded videos</p>\
     </video><p>after</p></body></html>";

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

fn serve_and_run(body: &str, args: &[&str]) -> Value {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body(body)
        .create();
    let url = server.url();
    let mut argv = vec![url.as_str()];
    argv.extend_from_slice(args);
    let (code, out) = run_capture(&argv);
    assert_eq!(code, 0);
    serde_json::from_str(out.trim()).expect("envelope JSON")
}

/// Every `role` in the `ax` payload, in emission order.
fn ax_roles(v: &Value) -> Vec<String> {
    fn walk(v: &Value, out: &mut Vec<String>) {
        if let Value::Array(arr) = v {
            for item in arr {
                out.push(item["role"].as_str().unwrap_or("").to_string());
                walk(&item["children"], out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&v["out"], &mut out);
    out
}

/// Every emitted `tag` in the `bboxes` payload, in reading order.
fn bbox_tags(v: &Value) -> Vec<String> {
    v["out"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["tag"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn text_keeps_the_page_and_drops_the_fallback_in_both_recipes() {
    for args in [vec!["--out", "text"], vec!["--css", "--out", "text"]] {
        let v = serve_and_run(MEDIA, &args);
        assert_eq!(v["status"], "ok", "{args:?}");
        assert_eq!(v["out"], "before\nafter", "{args:?}");
    }
}

#[test]
fn ax_exposes_no_fallback_node_in_either_recipe() {
    // The two `<p>` siblings are the page's real paragraphs; the third, inside
    // the `<video>`, is not a node at all. `<video>` itself has no role
    // mapping, so it contributes none — and this change does not invent one.
    for args in [vec!["--out", "ax"], vec!["--css", "--out", "ax"]] {
        let roles = ax_roles(&serve_and_run(MEDIA, &args));
        assert_eq!(roles, vec!["paragraph", "paragraph"], "{args:?}");
    }
}

#[test]
fn geometry_gives_the_fallback_no_box() {
    // `bboxes` is one entry per *rendered* element: the media box may be
    // there, its fallback never is.
    for args in [vec!["--out", "bboxes"], vec!["--css", "--out", "bboxes"]] {
        let tags = bbox_tags(&serve_and_run(MEDIA, &args));
        assert_eq!(
            tags.iter().filter(|t| *t == "p").count(),
            2,
            "{args:?} {tags:?}"
        );
        for absent in ["source", "track"] {
            assert!(!tags.contains(&absent.to_string()), "{args:?} {tags:?}");
        }
    }
}

#[test]
fn a_script_that_moves_the_fallback_out_of_the_media_element_renders_it() {
    // The rule keys on the live parent, so a DOM mutation is read with no
    // extra seam — the fallback is content once it is no longer fallback.
    let page = "<html><body><h1>page</h1><video><p id=f>rescued</p></video>\
         <script>document.body.appendChild(document.getElementById('f'));</script>\
         </body></html>";
    assert_eq!(
        serve_and_run(page, &["--css", "--out", "text"])["out"],
        "page"
    );
    assert_eq!(
        serve_and_run(page, &["--css", "--js", "--out", "text"])["out"],
        "page\nrescued"
    );
}
