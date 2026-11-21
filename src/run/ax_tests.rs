//! `--out ax` end-to-end: children surface in DOM order with or without
//! `--css`. Chrome's AX tree does not follow flex `order`/`*-reverse`
//! (measured, Chrome 156, `bl-7430`), so the view builds no layout.

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
fn ax_keeps_dom_order_in_a_reversed_flex_row() {
    for args in [&["--out", "ax"][..], &["--css", "--out", "ax"]] {
        let v = serve_ax(FLEX_NAV, args);
        assert_eq!(v["view"], "ax");
        assert_eq!(link_names(&v), vec!["A", "B"]);
    }
}
