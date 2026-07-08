//! `--css --out ax` end-to-end: a flex container's children surface in visual
//! reading order (the 3.7 refinement), driven by the layout `run.rs` builds for
//! `ax` only under `--css`. Without `--css` no layout is built, so the flex
//! `<style>` is inert and source order stands — the pre-Phase-3 behaviour.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

fn serve_ax(body: &str, args: &[&str]) -> Value {
    let mut server = mockito::Server::new();
    let _m = server.mock("GET", "/").with_status(200).with_body(body).create();
    let url = server.url();
    let mut argv = vec![url.as_str()];
    argv.extend_from_slice(args);
    let (code, out) = run_capture(&argv);
    assert_eq!(code, 0);
    serde_json::from_str(out.trim()).expect("envelope JSON")
}

/// Names of every `link` node in the `ax` payload, in emission order.
fn link_names(v: &Value) -> Vec<String> {
    fn walk(v: &Value, out: &mut Vec<String>) {
        if let Value::Array(arr) = v {
            for item in arr {
                if item["role"] == "link" {
                    out.push(item["name"].as_str().unwrap_or("").to_string());
                }
                walk(&item["children"], out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&v["out"], &mut out);
    out
}

const FLEX_NAV: &str = "<style>nav{display:flex;flex-direction:row-reverse}</style>\
     <nav><a href=\"/1\">A</a><a href=\"/2\">B</a></nav>";

#[test]
fn ax_without_css_keeps_source_order() {
    // No `--css`: no layout is built, the flex `<style>` is inert, links read in
    // markup order.
    let v = serve_ax(FLEX_NAV, &["--out", "ax"]);
    assert_eq!(v["view"], "ax");
    assert_eq!(link_names(&v), vec!["A", "B"]);
}

#[test]
fn ax_with_css_reorders_flex_children() {
    // `--css` builds the layout for `ax`; the row-reverse nav emits its links in
    // reversed reading order.
    let v = serve_ax(FLEX_NAV, &["--css", "--out", "ax"]);
    assert_eq!(v["view"], "ax");
    assert_eq!(link_names(&v), vec!["B", "A"]);
}
