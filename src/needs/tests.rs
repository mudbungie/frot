use super::*;

fn parse(html: &str) -> Document {
    Document::parse(html)
}

#[test]
fn dom_view_never_signals_needs() {
    let doc = parse("<html><body></body><script src=app.js></script></html>");
    assert!(detect(View::Dom, &doc, None).is_empty());
}

#[test]
fn meta_view_never_signals_needs() {
    let doc = parse("<html><body></body><script src=app.js></script></html>");
    assert!(detect(View::Meta, &doc, None).is_empty());
}

#[test]
fn bboxes_view_never_signals_needs_in_phase_0() {
    let doc = parse("<html><body></body><script></script></html>");
    assert!(detect(View::Bboxes, &doc, None).is_empty());
}

#[test]
fn empty_spa_shell_with_script_signals_js() {
    let doc = parse("<html><body><div id=root></div></body><script src=app.js></script></html>");
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
    assert_eq!(detect(View::Links, &doc, None), vec![NeedsKind::Js]);
    assert_eq!(detect(View::Forms, &doc, None), vec![NeedsKind::Js]);
    assert_eq!(detect(View::Ax, &doc, None), vec![NeedsKind::Js]);
}

#[test]
fn fully_rendered_page_does_not_signal_needs() {
    let doc = parse("<html><body><h1>Hello</h1><p>This is content.</p></body></html>");
    assert!(detect(View::Text, &doc, None).is_empty());
}

#[test]
fn empty_body_without_scripts_does_not_signal_js() {
    let doc = parse("<html><body></body></html>");
    assert!(detect(View::Text, &doc, None).is_empty());
}

#[test]
fn page_with_scripts_and_text_does_not_signal_js() {
    let doc = parse("<html><body><p>real content</p><script src=app.js></script></body></html>");
    assert!(detect(View::Text, &doc, None).is_empty());
}

#[test]
fn empty_document_does_not_signal_js() {
    // Document::default() has no body at all (no roots).
    let doc = Document::default();
    assert!(detect(View::Text, &doc, None).is_empty());
}

#[test]
fn parser_injected_empty_body_with_only_head_script_signals_js() {
    // html5ever injects <body></body>; the empty body + head script combo is
    // exactly the SPA-shell pattern the heuristic is meant to catch.
    let doc = parse("<html><script></script></html>");
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
}

#[test]
fn scaffold_of_empty_boxes_signals_js_regardless_of_size() {
    // bl-e22e: a large tree of empty divs is the same fact as a lone mount
    // div — nothing renderable. The retired `<= 3 elements` guard read this
    // as a rendered page.
    let doc = parse(
        "<html><body><div></div><div></div><div></div><div></div><div></div></body><script></script></html>",
    );
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
    assert_eq!(detect(View::Ax, &doc, None), vec![NeedsKind::Js]);
}

#[test]
fn labels_clear_non_text_views_but_not_the_text_view() {
    // View sensitivity (needs.md §4): `alt`/`aria-label` are content the AX,
    // links and forms views can express, so they clear starvation there; the
    // text view can never carry them, so it stays flagged — the documented
    // residual, erring toward flagging.
    let doc = parse(
        "<html><body><a href=/home><img src=logo.png alt=Home></a>\
         <button aria-label=Search></button>\
         <script src=app.js></script></body></html>",
    );
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
    assert!(detect(View::Ax, &doc, None).is_empty());
    assert!(detect(View::Links, &doc, None).is_empty());
    assert!(detect(View::Forms, &doc, None).is_empty());
}

#[test]
fn empty_alt_is_the_decorative_marker_not_a_label() {
    let doc = parse(
        "<html><body><img src=x.png alt=\"\"><div aria-label=\" \"></div>\
         <script src=app.js></script></body></html>",
    );
    assert_eq!(detect(View::Ax, &doc, None), vec![NeedsKind::Js]);
}

#[test]
fn labels_inside_chrome_are_boilerplate_not_content() {
    // Chrome subtrees are set aside for labels exactly as for text.
    let doc = parse(
        "<html><body><div id=root></div>\
         <nav><a href=/ aria-label=Home></a></nav>\
         <script src=app.js></script></body></html>",
    );
    assert_eq!(detect(View::Ax, &doc, None), vec![NeedsKind::Js]);
}

#[test]
fn whitespace_only_body_with_script_signals_js() {
    let doc = parse("<html><body>   \n   </body><script></script></html>");
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
}

#[test]
fn inline_script_source_is_not_content_so_shell_signals_js() {
    // A `<script>`'s own source is set aside, not counted as body text: an empty
    // mount div beside an inline bootstrap is still a starved shell.
    let doc = parse("<html><body><div id=root></div><script>var x = 1;</script></body></html>");
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
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
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
}

// --- Synthetic real-world shapes (tests/fixtures/needs) ---------------------
// The trial corpus was ephemeral; these stand in for it. Each is hand-authored
// to a real page's *structure* — no third-party markup or prose is reproduced
// (`tests/fixtures/needs/README.md` records which page each shape models, and
// `tests/fixtures/NOTICE.md` the licensing rule that keeps this directory
// synthetic). The shells carry static chrome (a todo footer, a masthead header
// + SEO h1) around an empty mount region — the case the body-empty heuristic
// used to miss.

const TODO_SHELL: &str = include_str!("../../tests/fixtures/needs/todo-spa-shell.html");
const CHAT_SCAFFOLD: &str = include_str!("../../tests/fixtures/needs/chat-app-scaffold.html");
const CANVAS_SHELL: &str = include_str!("../../tests/fixtures/needs/canvas-app-shell.html");
const ENCYCLOPEDIA: &str = include_str!("../../tests/fixtures/needs/encyclopedia-article.html");
const AGGREGATOR: &str = include_str!("../../tests/fixtures/needs/link-aggregator-listing.html");
const NEWS: &str = include_str!("../../tests/fixtures/needs/news-article.html");

/// Every content-dependent view must agree the page needs js.
fn needs_js_for_all_content_views(html: &str) {
    let doc = parse(html);
    for view in [View::Text, View::Ax, View::Links, View::Forms] {
        assert_eq!(detect(view, &doc, None), vec![NeedsKind::Js], "{view:?}");
    }
}

/// No content-dependent view may flag the page.
fn stays_ok_for_all_content_views(html: &str) {
    let doc = parse(html);
    for view in [View::Text, View::Ax, View::Links, View::Forms] {
        assert!(detect(view, &doc, None).is_empty(), "{view:?}");
    }
}

#[test]
fn todo_shell_signals_js() {
    // Empty `section.taskapp` + a static `footer.info` (chrome) + scripts.
    needs_js_for_all_content_views(TODO_SHELL);
}

#[test]
fn canvas_shell_signals_js() {
    // `<header>` masthead with an SEO `<h1>` (chrome) around an empty root div.
    needs_js_for_all_content_views(CANVAS_SHELL);
}

#[test]
fn chat_scaffold_signals_js() {
    // bl-e22e (field trial 2026-07-19): dozens of empty divs plus a
    // `<defs>`-only svg sprite sheet, zero text, zero labels. The retired
    // element-count guard reported this shape as an `ok` impression.
    needs_js_for_all_content_views(CHAT_SCAFFOLD);
}

#[test]
fn dead_app_after_js_still_signals_js() {
    // Post-settle re-detection runs `detect` on the post-JS document. When the
    // shim ran but rendered nothing, that document is byte-for-byte the shell —
    // so the signal must survive, honestly reporting "needs more js than frot
    // can give" rather than an empty impression that looks complete.
    needs_js_for_all_content_views(TODO_SHELL);
    needs_js_for_all_content_views(CANVAS_SHELL);
}

#[test]
fn encyclopedia_article_stays_ok() {
    stays_ok_for_all_content_views(ENCYCLOPEDIA);
}

#[test]
fn aggregator_listing_stays_ok() {
    stays_ok_for_all_content_views(AGGREGATOR);
}

#[test]
fn news_article_stays_ok() {
    stays_ok_for_all_content_views(NEWS);
}

// --- Transport-declared deferral (needs.md §3) ------------------------------

fn hdr(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect()
}

#[test]
fn retry_after_declares_a_challenge_whatever_its_value() {
    assert!(challenge(&hdr(&[("retry-after", "0")])));
    assert!(challenge(&hdr(&[("Retry-After", "120")])));
}

#[test]
fn cf_mitigated_challenge_declares_a_challenge_case_insensitively() {
    assert!(challenge(&hdr(&[("cf-mitigated", "challenge")])));
    assert!(challenge(&hdr(&[("CF-Mitigated", " Challenge ")])));
}

#[test]
fn other_cf_mitigated_values_are_not_a_challenge() {
    // Cloudflare uses the header for other mitigations; only the challenge
    // declaration means "the page was never served".
    assert!(!challenge(&hdr(&[("cf-mitigated", "block")])));
}

#[test]
fn amzn_waf_challenge_declares_a_challenge_case_insensitively() {
    // Measured 2026-07-22 (bl-7e34): www.amazon.com answers frot's exact
    // request headers with 202 + `x-amzn-waf-action: challenge` + an AWS WAF
    // `challenge.js` body. The declaration, not the empty-looking body, is
    // the fact.
    assert!(challenge(&hdr(&[("x-amzn-waf-action", "challenge")])));
    assert!(challenge(&hdr(&[("X-Amzn-Waf-Action", " Challenge ")])));
}

#[test]
fn other_amzn_waf_actions_are_not_a_challenge() {
    // AWS WAF also spells `captcha` / `block` here; only `challenge` is the
    // declared deferral this detector recognizes. (`captcha` at a 2xx would
    // fall through — it is not a deferral the server says to retry, and
    // inventing a verdict for an unmeasured shape is guessing.)
    assert!(!challenge(&hdr(&[("x-amzn-waf-action", "block")])));
}

#[test]
fn ordinary_response_headers_declare_nothing() {
    assert!(!challenge(&hdr(&[("content-type", "text/html")])));
    assert!(!challenge(&[]));
}
