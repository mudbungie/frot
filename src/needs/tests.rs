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
