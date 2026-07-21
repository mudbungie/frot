//! `--out bboxes` end-to-end: no longer a usage error (Phase 3), it exits 0
//! with a reading-order box array. Styles source is `--css`-consistent — a
//! `<style>` block is honored only under `--css` (`compute_with`), ignored
//! otherwise (`compute_bare`), so the flex reorder it declares fires only then.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn parse_envelope(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
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
    let (code, out, _) = run_capture(&argv);
    assert_eq!(code, 0);
    parse_envelope(&out)
}

/// Source ordinals of every emitted `<i>`, in reading order.
fn italic_is(v: &Value) -> Vec<i64> {
    v["out"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["tag"] == "i")
        .map(|e| e["i"].as_i64().unwrap())
        .collect()
}

#[test]
fn bboxes_view_exits_zero_with_reading_order_array() {
    let v = serve_and_run("<h1>Hi</h1><p>Body</p>", &["--out", "bboxes"]);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["view"], "bboxes");
    let arr = v["out"].as_array().expect("out is array");
    let h1 = arr.iter().find(|e| e["tag"] == "h1").expect("h1 box");
    assert_eq!(h1["text"], "Hi");
    assert_eq!(h1["rect"]["w"], 1280);
}

const FLEX_STYLE: &str =
    "<style>div{display:flex;flex-direction:row-reverse}</style><div><i>a</i><i>b</i></div>";

#[test]
fn bboxes_without_css_ignores_style_block() {
    // `compute_bare` drops `<style>` → div is block flow → source order.
    let v = serve_and_run(FLEX_STYLE, &["--out", "bboxes"]);
    let is = italic_is(&v);
    assert_eq!(is, vec![is[0], is[0] + 1]);
}

#[test]
fn bboxes_with_css_reorders_via_style_block() {
    // `--css` applies `<style>` → flex row-reverse → children reversed, so the
    // second source element (larger `i`) is emitted first (i non-monotonic).
    let v = serve_and_run(FLEX_STYLE, &["--css", "--out", "bboxes"]);
    let is = italic_is(&v);
    assert_eq!(is, vec![is[1] + 1, is[1]]);
}
