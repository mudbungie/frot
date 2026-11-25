//! End-to-end `--images` (`bl-49f1`, js.md §7 "Images"): the flag reaches the
//! JS layer, an image request leaves frot only with it, and a script awaiting
//! the image proceeds into the impression.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    let text = |b: Vec<u8>| String::from_utf8(b).unwrap();
    (code, text(out), text(err))
}

/// A page that writes the image's outcome into its text once it ends.
const PAGE: &str = "<html><body><p id=o>waiting</p><img id=i src=/i.gif><script>\
    var i=document.getElementById('i');i.onload=i.onerror=function(e){\
    document.getElementById('o').textContent=e.type+' '+i.naturalWidth}</script></body></html>";

const GIF: &[u8] = b"GIF89a\x03\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff,\x00\x00\x00\x00\x03\x00\x01\x00\x00\x02\x02D\x01\x00;";

/// Serve [`PAGE`] and its image, expecting the image `hits` times; return the
/// `text` view of the page under `flags`.
fn text(flags: &[&str], hits: usize) -> String {
    let mut server = mockito::Server::new();
    let _p = server.mock("GET", "/").with_body(PAGE).create();
    let img = server
        .mock("GET", "/i.gif")
        .with_body(GIF)
        .expect(hits)
        .create();
    let url = server.url();
    let mut args = vec![url.as_str()];
    args.extend_from_slice(flags);
    let (code, out, _) = run_capture(&args);
    assert_eq!(code, 0);
    img.assert();
    let env: Value = serde_json::from_str(out.trim()).unwrap();
    env["out"].as_str().unwrap().to_string()
}

#[test]
fn with_images_the_awaiting_script_sees_the_real_image() {
    let out = text(&["--js", "--images", "--out", "text"], 1);
    assert!(out.contains("load 3"), "{out:?}");
}

#[test]
fn without_images_nothing_is_requested_and_the_image_fails() {
    let out = text(&["--js", "--out", "text"], 0);
    assert!(out.contains("error 0"), "{out:?}");
}

#[test]
fn images_without_js_is_a_usage_error() {
    let (code, out, err) = run_capture(&["https://example.com/", "--images", "--out", "text"]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(err.contains("--images requires --js"), "{err:?}");
}
