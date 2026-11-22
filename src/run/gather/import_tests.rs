//! `@import` through the gather: cascade order, resolution, the cycle and
//! depth bounds, the remote→local block, and `<style>` imports.

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

const GENEROUS: Duration = Duration::from_secs(30);

/// A CSS origin answering each path with `route(path)`; `None` hangs up
/// without a response (a failed fetch). Returns the origin and the log of
/// paths requested.
fn origin(route: fn(&str) -> Option<String>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let log = Arc::new(Mutex::new(Vec::new()));
    let seen = log.clone();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let seen = seen.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).unwrap_or(0);
                let head = String::from_utf8_lossy(&buf[..n]).into_owned();
                let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
                seen.lock().unwrap().push(path.clone());
                if let Some(body) = route(&path) {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: text/css\r\nConnection: close\r\n\
                         Content-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    );
                }
            });
        }
    });
    (base, log)
}

fn gather(base: &str, paths: &[&str]) -> Vec<String> {
    let hrefs: Vec<String> = paths.iter().map(|p| format!("{base}{p}")).collect();
    gather_within(&hrefs, base, &FetchSession::new(Vec::new()), GENEROUS)
}

fn tree(path: &str) -> Option<String> {
    Some(
        match path {
            "/a.css" => "@import 'b.css';@import url(dir/c.css);a{}",
            "/b.css" => "@import \"d.css\";b{}",
            "/d.css" => "d{}",
            "/dir/c.css" => "@import 'e.css';c{}",
            "/dir/e.css" => "e{}",
            "/z.css" => "z{}",
            _ => return None,
        }
        .to_string(),
    )
}

#[test]
fn an_imported_sheet_cascades_ahead_of_its_importer() {
    let (base, _) = origin(tree);
    let got = gather(&base, &["/a.css", "/z.css"]);
    let tail: Vec<&str> = got
        .iter()
        .map(|s| &s[s.rfind(';').map_or(0, |i| i + 1)..])
        .collect();
    // Depth-first, imports before the importing sheet's own rules; a relative
    // target resolves against the importing sheet (`dir/c.css` → `dir/e.css`).
    assert_eq!(tail, ["d{}", "b{}", "e{}", "c{}", "a{}", "z{}"]);
}

fn conditional(path: &str) -> Option<String> {
    Some(
        match path {
            "/a.css" => {
                "@import 'print.css' print;@import 'gone.css';\
                 @import 'file:///etc/passwd';@import '';@import 'screen.css' screen;a{}"
            }
            "/screen.css" => "s{}",
            "/print.css" => "p{}",
            _ => return None,
        }
        .to_string(),
    )
}

#[test]
fn only_applicable_reachable_remote_imports_are_gathered() {
    let (base, log) = origin(conditional);
    let got = gather(&base, &["/a.css"]);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0], "s{}");
    assert!(got[1].ends_with("a{}"));
    let mut asked = log.lock().unwrap().clone();
    asked.sort();
    // The print import is never requested, nor is the `file:` one (a remote
    // sheet cannot read local files) or the empty one; the failed fetch is
    // skipped without costing the sheet that imports it.
    assert_eq!(asked, ["/a.css", "/gone.css", "/screen.css"]);
}

fn cyclic(path: &str) -> Option<String> {
    Some(
        match path {
            "/a.css" => "@import 'b.css';@import 'a.css';a{}",
            "/b.css" => "@import 'a.css';b{}",
            _ => return None,
        }
        .to_string(),
    )
}

#[test]
fn an_import_cycle_contributes_each_sheet_once() {
    let (base, _) = origin(cyclic);
    let got = gather(&base, &["/a.css"]);
    assert_eq!(got.len(), 2);
    assert!(got[0].ends_with("b{}") && got[1].ends_with("a{}"));
}

fn endless(path: &str) -> Option<String> {
    let n: usize = path
        .trim_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()?;
    Some(format!("@import '{}.css';n{n}{{}}", n + 1))
}

#[test]
fn an_endless_import_chain_stops_at_the_depth_bound() {
    let (base, _) = origin(endless);
    let got = gather(&base, &["/0.css"]);
    assert_eq!(got.len(), IMPORT_DEPTH + 1);
    assert!(got[0].ends_with(&format!("n{IMPORT_DEPTH}{{}}")));
    assert!(got[IMPORT_DEPTH].ends_with("n0{}"));
}

#[test]
fn a_local_sheet_imports_its_sibling_when_the_page_is_local() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fidelity");
    let page = format!("file://{dir}/reach.html");
    let got = gather_within(
        &[format!("file://{dir}/reach.css")],
        &page,
        &FetchSession::new(Vec::new()),
        GENEROUS,
    );
    assert_eq!(got.len(), 2);
    assert!(got[0].contains(".imported"));
    assert!(got[1].contains("@import"));
}

#[test]
fn a_style_blocks_imports_join_the_links_in_document_order() {
    let doc = crate::dom::Document::parse(
        "<link rel=stylesheet href='/a.css'>\
         <style>@import 'b.css'; @import url(/c.css) print; p{}</style>\
         <style media=print>@import 'd.css';</style>\
         <link rel=stylesheet href='http://[bad'>\
         <link rel=stylesheet href='/e.css'>",
    );
    assert_eq!(
        external_hrefs(&doc, "http://example.com/dir/page"),
        [
            "http://example.com/a.css",
            "http://example.com/dir/b.css",
            "http://example.com/e.css",
        ]
    );
}
