//! `<link media=…>` end-to-end: a print-only sheet must not reach the screen
//! cascade, and a sheet whose media *does* apply must. Pins the w3.org shape
//! from bl-4f0a — `core.css media=screen`, `advanced.css` with a media-query
//! list, `print.css media=print` whose `#global-nav ul{display:none!important}`
//! was deleting the site's navigation from `--out text`.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

const PAGE: &str = "<html><head>\
     <link rel=stylesheet href='/core.css' media='screen'>\
     <link rel=stylesheet href='/advanced.css' media='screen and (min-width: 481px), print'>\
     <link rel=stylesheet href='/print.css' media='MEDIA'>\
     </head><body>\
     <div id=global-nav><ul><li>Standards</li></ul></div>\
     <p id=lang>English</p><p id=adv></p>\
     </body></html>";

/// Serve the three-sheet page with `print.css` linked at `print_media`, and
/// return its `--css --out text`.
fn text_with_print_media(print_media: &str) -> String {
    let mut server = mockito::Server::new();
    let sheets = [
        // A nested `@media print` inside a *matching* sheet must not apply
        // either — same authority, one level down.
        (
            "/core.css",
            "#global-nav ul{display:flex}@media print{#lang{display:none}}",
        ),
        ("/advanced.css", "#adv::before{content:\"ADV\"}"),
        (
            "/print.css",
            "#global-nav ul{display:none!important}#lang{display:none}",
        ),
    ];
    let _m: Vec<mockito::Mock> = sheets
        .iter()
        .map(|(path, body)| {
            server
                .mock("GET", *path)
                .with_status(200)
                .with_header("content-type", "text/css")
                .with_body(*body)
                .create()
        })
        .collect();
    let _page = server
        .mock("GET", "/")
        .with_status(200)
        .with_body(PAGE.replace("MEDIA", print_media))
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--css", "--out", "text"]);
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(out.trim()).expect("envelope JSON");
    v["out"].as_str().unwrap().to_string()
}

/// The print sheet does not participate at the 1280×720 screen viewport, so the
/// navigation and language links it hides survive — while the media-query-list
/// sheet that *does* match still applies its generated content.
#[test]
fn a_print_only_sheet_does_not_hide_screen_content() {
    let text = text_with_print_media("print");
    assert!(text.contains("Standards"), "nav lost: {text}");
    assert!(text.contains("English"), "language link lost: {text}");
    assert!(text.contains("ADV"), "matching sheet not applied: {text}");
}

/// The negative control: the very same sheet, linked `media=screen`, does hide
/// them — so the test above pins the `media` attribute, not an unfetched sheet.
#[test]
fn the_same_sheet_linked_for_screen_does_hide_it() {
    let text = text_with_print_media("screen");
    assert!(!text.contains("Standards"), "nav survived: {text}");
    assert!(!text.contains("English"), "language link survived: {text}");
}
