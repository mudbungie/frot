use super::*;

fn t(html: &str) -> String {
    text(&Document::parse(html), None)
}

fn t_css(html: &str) -> String {
    let doc = Document::parse(html);
    let s = crate::css::compute(&doc);
    text(&doc, Some(&s))
}

#[test]
fn empty_document_renders_empty() {
    assert_eq!(t(""), "");
}

#[test]
fn plain_text_is_returned_verbatim_when_no_whitespace_runs() {
    assert_eq!(t("<p>hello</p>"), "hello");
}

#[test]
fn whitespace_runs_collapse_outside_pre() {
    assert_eq!(t("<p>hello   world</p>"), "hello world");
    assert_eq!(t("<p>  hello\n\n\tworld  </p>"), "hello world");
}

#[test]
fn pre_preserves_whitespace() {
    let got = t("<pre>  a\n  b</pre>");
    assert_eq!(got, "  a\n  b");
}

#[test]
fn br_emits_newline() {
    assert_eq!(t("<p>hello<br>world</p>"), "hello\nworld");
}

#[test]
fn script_and_style_subtrees_skipped() {
    let html = "<p>a</p><script>var x = 1;</script><style>p{color:red}</style><p>b</p>";
    let got = t(html);
    assert!(got.contains("a"));
    assert!(got.contains("b"));
    assert!(!got.contains("var x"));
    assert!(!got.contains("color"));
}

#[test]
fn template_is_skipped() {
    let got = t("<p>shown</p><template><p>hidden</p></template>");
    assert!(got.contains("shown"));
    assert!(!got.contains("hidden"));
}

#[test]
fn block_boundaries_become_newlines() {
    let got = t("<p>first</p><p>second</p>");
    assert!(got.contains("first"));
    assert!(got.contains("second"));
    assert!(got.contains("\n"));
}

#[test]
fn inline_elements_dont_introduce_breaks() {
    assert_eq!(t("<p>hello <span>world</span></p>"), "hello world");
}

#[test]
fn nested_blocks_dont_double_break() {
    let got = t("<div><p>a</p><p>b</p></div>");
    assert!(!got.contains("\n\n\n"));
    assert!(got.contains("a"));
    assert!(got.contains("b"));
}

#[test]
fn comments_and_doctype_are_skipped() {
    let got = t("<!doctype html><html><body><!-- hidden -->visible</body></html>");
    assert!(!got.contains("hidden"));
    assert!(got.contains("visible"));
}

#[test]
fn trailing_whitespace_trimmed() {
    let got = t("<p>hello</p>     ");
    assert!(!got.ends_with(' '));
    assert!(!got.ends_with('\n'));
}

#[test]
fn br_inside_pre_emits_real_newline() {
    let got = t("<pre>a<br>b</pre>");
    assert!(got.contains("a"));
    assert!(got.contains("b"));
    assert!(got.contains('\n'));
}

#[test]
fn block_break_inside_pre_does_nothing_extra() {
    let got = t("<pre>line1\n<p>line2</p>line3</pre>");
    assert!(got.contains("line1"));
    assert!(got.contains("line2"));
    assert!(got.contains("line3"));
}

#[test]
fn leading_whitespace_does_not_inject_space() {
    let got = t("  <p>hi</p>");
    assert_eq!(got, "hi");
}

#[test]
fn multiple_brs_emit_multiple_newlines() {
    let got = t("<p>a<br><br>b</p>");
    let lines: Vec<&str> = got.split('\n').collect();
    assert!(lines.len() >= 2);
    assert!(got.contains("a"));
    assert!(got.contains("b"));
}

#[test]
fn nested_inline_collapses_correctly() {
    let got = t("<p>a <b>b <i>c</i> d</b> e</p>");
    assert_eq!(got, "a b c d e");
}

#[test]
fn leading_whitespace_in_inline_at_doc_start_dropped() {
    let got = t("<html><body><span>   hello</span></body></html>");
    assert_eq!(got, "hello");
}

/// `push_text` and `flush_pending` carry no "is `out` empty?" guard, because
/// the root block break always fires first (see [`State::push_text`]'s docs).
/// That rests entirely on `html`/`body` being block tags — `tags.rs` warns its
/// membership is load-bearing, and this is the second consumer it means. Drop
/// either name from `BLOCK_TAGS` and a leading space reappears in `--out text`,
/// which `finish` does not trim; so pin the membership *and* the behaviour it
/// buys, across every shape that can put whitespace before the first character.
#[test]
fn text_never_starts_with_whitespace() {
    assert!(crate::tags::is_block("html"), "html must stay a block tag");
    assert!(crate::tags::is_block("body"), "body must stay a block tag");
    for html in [
        "  <p>hi</p>",
        "<html><body><span>   hello</span></body></html>",
        "<html>\n\t<body>\n<div>\n  x</div></body></html>",
        "<span> a</span>",
        "<html><body>   <b> </b> <i>  q</i></body></html>",
    ] {
        let got = t(html);
        assert!(
            !got.starts_with([' ', '\n']),
            "leading whitespace survived for {html:?}: {got:?}"
        );
    }
}

#[test]
fn trailing_whitespace_from_pre_is_trimmed() {
    let got = t("<pre>hello </pre>");
    assert_eq!(got, "hello");
}

#[test]
fn css_display_none_removes_subtree() {
    assert_eq!(
        t_css("<p>keep</p><p style='display:none'>gone<span>x</span></p>"),
        "keep"
    );
}

#[test]
fn css_visibility_hidden_drops_own_text_but_visible_child_returns() {
    let got = t_css(
        "<div style='visibility:hidden'>hide\
         <span style='visibility:visible'>show</span></div>",
    );
    assert!(!got.contains("hide"), "got {got:?}");
    assert!(got.contains("show"), "got {got:?}");
}

#[test]
fn css_generated_content_is_emitted_inline() {
    let got = t_css("<style>a::before{content:'» '}a::after{content:' «'}</style><a>link</a>");
    assert_eq!(got, "» link «");
}

#[test]
fn css_hidden_element_suppresses_its_generated_content_and_text() {
    let got = t_css("<style>p::before{content:'X'}</style><p style='visibility:hidden'>y</p>");
    assert_eq!(got, "");
}

#[test]
fn hidden_attribute_subtree_is_dropped_under_css_but_raw_text_keeps_it() {
    // The UA rule `[hidden] { display: none }` reaches the text view through the
    // cascade (`bl-eeb4`). Without `--css` there is no cascade and the raw
    // source-order dump is unchanged — `--css` is what "apply CSS" means.
    let html = "<p>keep</p><div hidden><span>Opens in a new tab</span></div>";
    assert_eq!(t_css(html), "keep");
    assert!(t(html).contains("Opens in a new tab"));
}

#[test]
fn css_with_no_rules_matches_raw_extraction() {
    let html = "<div><p>a</p><p>b</p></div>";
    assert_eq!(t_css(html), t(html));
}

#[test]
fn closed_details_emits_only_its_summary_under_css() {
    // `bl-74a6`: the disclosure body is not painted until someone clicks, and
    // frot never clicks. Raw text goes with the elements — it is concealed by
    // the same box.
    let html =
        "<details><summary>Options</summary><p>Use options to customise…</p>and raw</details>";
    assert_eq!(t_css(html), "Options");
    // Opened, the whole disclosure is content again.
    assert_eq!(
        t_css("<details open><summary>Options</summary><p>Use options…</p></details>"),
        "Options\nUse options…"
    );
}

#[test]
fn raw_text_without_css_keeps_its_source_order_dump() {
    // No `--css` means no cascade to consult: `text` stays the documented
    // source-order dump, exactly as it does for `[hidden]` (`bl-eeb4`).
    assert_eq!(
        t("<details><summary>Options</summary><p>body</p></details>"),
        "Options\nbody"
    );
}

#[test]
fn media_fallback_never_reaches_text() {
    // `bl-0f83`: fallback is for a UA without `<video>`; frot has it. Both the
    // element-wrapped and the bare-prose shapes go — and, unlike a CSS-hidden
    // subtree, in *both* recipes: the content model says so with no cascade
    // involved, so `--css` is not what makes it true.
    let wrapped =
        "<p>before</p><video><source src=a.mp4><p>Sorry, no embedded videos</p></video><p>after</p>";
    assert_eq!(t_css(wrapped), "before\nafter");
    assert_eq!(t(wrapped), "before\nafter");
    for html in [
        "<video>Sorry, your browser does not support embedded videos</video>",
        "<audio>no audio</audio>",
    ] {
        assert_eq!(t_css(html), "", "{html}");
        assert_eq!(t(html), "", "{html}");
    }
}
