use super::*;

fn parse(html: &str) -> Document {
    Document::parse(html)
}

#[test]
fn dom_view_never_signals_needs() {
    let doc = parse("<html><body></body><script src=app.js></script></html>");
    assert!(detect(View::Dom, &doc).is_empty());
}

#[test]
fn meta_view_never_signals_needs() {
    let doc = parse("<html><body></body><script src=app.js></script></html>");
    assert!(detect(View::Meta, &doc).is_empty());
}

#[test]
fn bboxes_view_never_signals_needs_in_phase_0() {
    let doc = parse("<html><body></body><script></script></html>");
    assert!(detect(View::Bboxes, &doc).is_empty());
}

#[test]
fn empty_spa_shell_with_script_signals_js() {
    let doc = parse("<html><body><div id=root></div></body><script src=app.js></script></html>");
    assert_eq!(detect(View::Text, &doc), vec![NeedsKind::Js]);
    assert_eq!(detect(View::Links, &doc), vec![NeedsKind::Js]);
    assert_eq!(detect(View::Forms, &doc), vec![NeedsKind::Js]);
    assert_eq!(detect(View::Ax, &doc), vec![NeedsKind::Js]);
}

#[test]
fn fully_rendered_page_does_not_signal_needs() {
    let doc = parse("<html><body><h1>Hello</h1><p>This is content.</p></body></html>");
    assert!(detect(View::Text, &doc).is_empty());
}

#[test]
fn empty_body_without_scripts_does_not_signal_js() {
    let doc = parse("<html><body></body></html>");
    assert!(detect(View::Text, &doc).is_empty());
}

#[test]
fn page_with_scripts_and_text_does_not_signal_js() {
    let doc =
        parse("<html><body><p>real content</p><script src=app.js></script></body></html>");
    assert!(detect(View::Text, &doc).is_empty());
}

#[test]
fn empty_document_does_not_signal_js() {
    // Document::default() has no body at all (no roots).
    let doc = Document::default();
    assert!(detect(View::Text, &doc).is_empty());
}

#[test]
fn parser_injected_empty_body_with_only_head_script_signals_js() {
    // html5ever injects <body></body>; the empty body + head script combo is
    // exactly the SPA-shell pattern the heuristic is meant to catch.
    let doc = parse("<html><script></script></html>");
    assert_eq!(detect(View::Text, &doc), vec![NeedsKind::Js]);
}

#[test]
fn body_with_many_empty_descendants_does_not_signal_js() {
    let doc = parse(
        "<html><body><div></div><div></div><div></div><div></div><div></div></body><script></script></html>",
    );
    assert!(detect(View::Text, &doc).is_empty());
}

#[test]
fn whitespace_only_body_with_script_signals_js() {
    let doc = parse("<html><body>   \n   </body><script></script></html>");
    assert_eq!(detect(View::Text, &doc), vec![NeedsKind::Js]);
}

#[test]
fn inline_script_source_is_not_content_so_shell_signals_js() {
    // A `<script>`'s own source is set aside, not counted as body text: an empty
    // mount div beside an inline bootstrap is still a starved shell.
    let doc = parse("<html><body><div id=root></div><script>var x = 1;</script></body></html>");
    assert_eq!(detect(View::Text, &doc), vec![NeedsKind::Js]);
}

#[test]
fn content_inside_chrome_only_still_signals_js() {
    // Text that lives only in chrome (a footer) is boilerplate, not content;
    // the body around the empty mount is still starved.
    let doc = parse(
        "<html><body><div id=root></div>\
         <footer><p>Created by someone</p></footer>\
         <script src=app.js></script></body></html>",
    );
    assert_eq!(detect(View::Text, &doc), vec![NeedsKind::Js]);
}

// --- Vendored real-world shells and content pages (tests/fixtures/needs) ----
// The trial corpus was ephemeral; these stand in for it. The two SPA shells
// carry static chrome (a todomvc footer, an excalidraw header + SEO h1) around
// an empty mount region — the case the body-empty heuristic used to miss.

const TODOMVC: &str = include_str!("../../tests/fixtures/needs/todomvc.html");
const EXCALIDRAW: &str = include_str!("../../tests/fixtures/needs/excalidraw.html");
const WIKIPEDIA: &str = include_str!("../../tests/fixtures/needs/wikipedia.html");
const HACKERNEWS: &str = include_str!("../../tests/fixtures/needs/hackernews.html");
const GUARDIAN: &str = include_str!("../../tests/fixtures/needs/guardian.html");

/// Every content-dependent view must agree the page needs js.
fn needs_js_for_all_content_views(html: &str) {
    let doc = parse(html);
    for view in [View::Text, View::Ax, View::Links, View::Forms] {
        assert_eq!(detect(view, &doc), vec![NeedsKind::Js], "{view:?}");
    }
}

/// No content-dependent view may flag the page.
fn stays_ok_for_all_content_views(html: &str) {
    let doc = parse(html);
    for view in [View::Text, View::Ax, View::Links, View::Forms] {
        assert!(detect(view, &doc).is_empty(), "{view:?}");
    }
}

#[test]
fn todomvc_shell_signals_js() {
    // Empty `section.todoapp` + a static `footer.info` (chrome) + scripts.
    needs_js_for_all_content_views(TODOMVC);
}

#[test]
fn excalidraw_shell_signals_js() {
    // `<header>` masthead with an SEO `<h1>` (chrome) around an empty root div.
    needs_js_for_all_content_views(EXCALIDRAW);
}

#[test]
fn dead_app_after_js_still_signals_js() {
    // Post-settle re-detection runs `detect` on the post-JS document. When the
    // shim ran but rendered nothing, that document is byte-for-byte the shell —
    // so the signal must survive, honestly reporting "needs more js than frot
    // can give" rather than an empty impression that looks complete.
    needs_js_for_all_content_views(TODOMVC);
    needs_js_for_all_content_views(EXCALIDRAW);
}

#[test]
fn wikipedia_article_stays_ok() {
    stays_ok_for_all_content_views(WIKIPEDIA);
}

#[test]
fn hackernews_listing_stays_ok() {
    stays_ok_for_all_content_views(HACKERNEWS);
}

#[test]
fn guardian_article_stays_ok() {
    stays_ok_for_all_content_views(GUARDIAN);
}
