//! `--images` (`bl-49f1`, js.md §7 "Images"): an `<img>` request leaves frot
//! with Firefox 153.0esr's image headers, is fetched once in a wave, and ends in
//! `load` with the real size or `error` — measured against that binary
//! (identity.md §3.20 image block). Served by `mockito`; nothing leaves the box.

use mockito::{Matcher, Mock, Server, ServerGuard};

use super::super::engine::{Clock, NetBudget};
use super::{frozen_bounds, run, Env, Report, StyleSource};
use crate::dom::{Document, NodeKind};
use crate::fetch::FetchSession;
use crate::js::JsStop;

/// A 1×1 GIF and a 2×3 PNG.
const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff!\xf9\x04\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x01D\x00;";
const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAADCAIAAAA2iEnWAAAAEUlEQVR4nGP4z8AARAwMDAwAPtgC/n3ICKQAAAAASUVORK5CYII=";
const ACCEPT: &str = "image/avif,image/webp,image/png,image/svg+xml,image/*;q=0.8,*/*;q=0.5";

fn env(url: &str) -> Env {
    Env {
        url: url.into(),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
        images: true,
        referrer: String::new(),
    }
}

/// `<body data-out>` once the run is over.
fn out(doc: &Document) -> String {
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    el.attr("data-out").unwrap_or_default().to_string()
}

/// Run `body` (markup + script) at the server's `/page`, the script logging
/// with `L(...)` into `data-out` at a far timer.
fn page(server: &ServerGuard, body: &str) -> (String, Report) {
    let html = format!(
        "<body style=margin:0><script>var log=[];function L(){{log.push([].slice.call(arguments).join(':'))}}\
         function watch(i,k){{i.onload=i.onerror=function(e){{L(k,e.type,i.complete,i.naturalWidth,i.naturalHeight)}}}}\
         setTimeout(function(){{document.body.setAttribute('data-out',log.join(' '))}},9000)</script>{body}</body>"
    );
    // `--css` geometry, so a lazy image's inline `margin-top` places it.
    let (doc, report) = run(
        Document::parse(&html),
        StyleSource::Authored(Vec::new()),
        env(&format!("{}/page", server.url())),
        &FetchSession::new(Vec::new()),
        frozen_bounds(),
    );
    (out(&doc), report)
}

/// An image endpoint expecting `hits` requests with Firefox's image headers and
/// the given `priority`.
fn img(server: &mut Server, path: &str, priority: &str, body: &[u8], hits: usize) -> Mock {
    server
        .mock("GET", path)
        .match_header("accept", ACCEPT)
        .match_header("sec-fetch-dest", "image")
        .match_header("sec-fetch-mode", "no-cors")
        .match_header("sec-fetch-site", "same-origin")
        .match_header("priority", priority)
        .match_header("referer", Matcher::Regex("/page$".into()))
        .match_header("range", Matcher::Missing)
        .with_header("content-type", "image/png")
        .with_body(body)
        .expect(hits)
        .create()
}

fn png() -> Vec<u8> {
    crate::js::subfetch::data::bytes(&format!("image/png;base64,{PNG}"))
        .unwrap()
        .1
}

#[test]
fn a_parsed_image_and_a_beacon_are_fetched_and_load_with_their_size() {
    let mut server = Server::new();
    // Parsed and in the document: `u=4, i`. A detached `new Image()` (a beacon):
    // `u=5, i`. The same URL twice is fetched once.
    let parsed = img(&mut server, "/a.png", "u=4, i", &png(), 1);
    let beacon = img(&mut server, "/p.gif?id=1", "u=5, i", GIF, 1);
    let (out, report) = page(
        &server,
        "<img id=a src=/a.png><img id=b src=/a.png><script>\
         watch(document.getElementById('a'),'a');watch(document.getElementById('b'),'b');\
         var p=new Image();watch(p,'p');p.src='/p.gif?id=1';\
         setTimeout(function(){var c=new Image();watch(c,'c');c.src='/a.png'},10)</script>",
    );
    parsed.assert();
    beacon.assert();
    assert_eq!(report.stopped, None);
    assert_eq!(
        out,
        "a:load:true:2:3 b:load:true:2:3 p:load:true:1:1 c:load:true:2:3"
    );
}

#[test]
fn the_body_decides_not_the_status() {
    let mut server = Server::new();
    // Measured: a 404 carrying a whole image loads; a cut one, or a non-image,
    // fails; SVG is SVG only when the response declares it.
    let _a = server
        .mock("GET", "/404.gif")
        .with_status(404)
        .with_body(GIF)
        .create();
    let _b = server
        .mock("GET", "/cut.gif")
        .with_body(&GIF[..20])
        .create();
    let _c = server.mock("GET", "/t.txt").with_body("hello").create();
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='4' height='5'/>";
    let _d = server
        .mock("GET", "/s.svg")
        .with_header("content-type", "image/svg+xml")
        .with_body(svg)
        .create();
    let _e = server.mock("GET", "/u.svg").with_body(svg).create();
    let (out, _) = page(
        &server,
        "<script>['404.gif','cut.gif','t.txt','s.svg','u.svg','http://127.0.0.1:9/x.png',\
         'file:///x.png','data:image/png;base64,@@@','data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP\
         ///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7'].forEach(function(u){var i=new Image();\
         watch(i,u.slice(-5));i.src=u})</script>",
    );
    // A refused connection, a remote page's `file:` image and a malformed
    // `data:` URL fail without a wave; a whole `data:` image loads without one.
    assert_eq!(
        out,
        "4.gif:load:true:1:1 t.gif:error:true:0:0 t.txt:error:true:0:0 \
         s.svg:load:true:4:5 u.svg:error:true:0:0 x.png:error:true:0:0 \
         x.png:error:true:0:0 4,@@@:error:true:0:0 BRAA7:load:true:1:1"
    );
}

#[test]
fn a_lazy_image_waits_for_the_viewport_margin() {
    let mut server = Server::new();
    // In view: released by the observer, at the lower priority. Far below:
    // never requested, still pending.
    let near = img(&mut server, "/near.png", "u=5, i", &png(), 1);
    let far = server.mock("GET", "/far.png").expect(0).create();
    // frot's layout parses no margins or heights (layout.md §6): paragraphs
    // place the far image, 100 lines down.
    let (out, _) = page(
        &server,
        &format!(
            "<p>x</p><img id=n loading=lazy src=/near.png>{}<img id=f loading=LAZY \
             src=/far.png><script>watch(document.getElementById('n'),'n');\
             var f=document.getElementById('f');L('top',f.getBoundingClientRect().top>1320);\
             setTimeout(function(){{L('f',f.complete,f.naturalWidth)}},8000)</script>",
            "<p>line</p>".repeat(100)
        ),
    );
    near.assert();
    far.assert();
    assert_eq!(out, "top:true n:load:true:2:3 f:false:0");
}

/// A `file:` page's images are files, read as bytes like any image body.
#[test]
fn a_local_page_loads_its_local_image() {
    let dir = std::env::temp_dir().join(format!("frot-imagefetch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.png"), png()).unwrap();
    let page = url::Url::from_file_path(dir.join("p.html")).unwrap();
    let (doc, _) = run(
        Document::parse(
            "<body><img id=a src=a.png><script>var a=document.getElementById('a');\
             a.onload=function(){document.body.setAttribute('data-out',a.naturalHeight)}</script></body>",
        ),
        StyleSource::Bare,
        env(page.as_str()),
        &FetchSession::new(Vec::new()),
        frozen_bounds(),
    );
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out(&doc), "3");
}

#[test]
fn an_image_the_budget_cut_off_fails_and_names_the_network() {
    let mut server = Server::new();
    let none = server.mock("GET", Matcher::Any).expect(0).create();
    // A budget already spent: the wave sends nothing, the image fails, and the
    // run says the network stopped it.
    let clock = Clock::manual();
    let mut bounds = frozen_bounds();
    bounds.net = NetBudget::on(clock.clone(), std::time::Duration::ZERO);
    let (doc, report) = run(
        Document::parse(
            "<body><img id=a src=/a.png><script>document.getElementById('a').onerror=\
             function(){document.body.setAttribute('data-out','error')}</script></body>",
        ),
        StyleSource::Bare,
        env(&format!("{}/page", server.url())),
        &FetchSession::new(Vec::new()),
        bounds,
    );
    none.assert();
    assert_eq!(out(&doc), "error");
    assert_eq!(report.stopped, Some(JsStop::Network));
}
