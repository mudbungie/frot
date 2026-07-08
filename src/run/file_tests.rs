//! End-to-end `file://` behavior: local documents render, relative hrefs
//! resolve against the file URL, local sheets apply under `--css`, and a
//! remote page can never pull a `file:` sheet.

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

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("frot-run-file-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn file_url(p: &std::path::Path) -> String {
    Url::from_file_path(p).unwrap().to_string()
}

#[test]
fn file_page_links_resolve_against_the_file_url() {
    let dir = tmp_dir("links");
    let page = dir.join("page.html");
    std::fs::write(&page, r#"<a href="other.html">next</a>"#).unwrap();
    let (code, out, _) = run_capture(&[&file_url(&page), "--out", "links"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    let href = v["out"][0]["href"].as_str().unwrap();
    assert!(href.starts_with("file://"), "href was {}", href);
    assert!(href.ends_with("/other.html"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn file_page_applies_local_stylesheet_under_css() {
    let dir = tmp_dir("css");
    std::fs::write(dir.join("hide.css"), ".hide{display:none}").unwrap();
    let page = dir.join("page.html");
    std::fs::write(
        &page,
        r#"<link rel="stylesheet" href="hide.css"><div class="hide">secret</div><p>visible</p>"#,
    )
    .unwrap();
    let (code, out, _) = run_capture(&[&file_url(&page), "--css", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    let text = v["out"].as_str().unwrap();
    assert!(!text.contains("secret"), "file sheet not applied: {}", text);
    assert!(text.contains("visible"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn file_page_omits_http_block() {
    let dir = tmp_dir("no-http");
    let page = dir.join("page.html");
    std::fs::write(&page, "<p>local</p>").unwrap();
    let (code, out, _) = run_capture(&[&file_url(&page), "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "local");
    // No HTTP response happened, so the http block is absent entirely.
    assert!(
        v.get("http").is_none(),
        "file:// carried an http block: {}",
        out
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn remote_page_cannot_pull_a_file_stylesheet() {
    let dir = tmp_dir("blocked");
    let sheet = dir.join("hide.css");
    std::fs::write(&sheet, ".hide{display:none}").unwrap();
    let mut server = mockito::Server::new();
    let body = format!(
        r#"<link rel="stylesheet" href="{}"><div class="hide">secret</div>"#,
        file_url(&sheet)
    );
    let _m = server.mock("GET", "/").with_body(body).create();
    let (code, out, _) = run_capture(&[&server.url(), "--css", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    let text = v["out"].as_str().unwrap();
    // The file: sheet must be skipped, so "secret" stays visible.
    assert!(text.contains("secret"), "file sheet was applied: {}", text);
    std::fs::remove_dir_all(&dir).unwrap();
}
