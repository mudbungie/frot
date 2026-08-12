use super::*;

fn d(name: &str, value: &str, important: bool) -> Decl {
    Decl {
        name: name.into(),
        value: value.into(),
        important,
    }
}

#[test]
fn basic_rule_and_decls() {
    let s = Stylesheet::parse("p { color: red; margin: 0 }");
    assert_eq!(s.rules.len(), 1);
    assert_eq!(s.rules[0].selectors.len(), 1);
    assert_eq!(
        s.rules[0].decls,
        vec![d("color", "red", false), d("margin", "0", false)]
    );
}

#[test]
fn comments_are_stripped_including_unterminated() {
    let s = Stylesheet::parse("a/*x*/b{c:d} /* trailing");
    assert_eq!(s.rules.len(), 1);
    assert_eq!(s.rules[0].decls, vec![d("c", "d", false)]);
    // a bare slash that is not a comment opener survives (here in a value).
    let s = Stylesheet::parse("a{b:c/d}");
    assert_eq!(s.rules[0].decls, vec![d("b", "c/d", false)]);
}

#[test]
fn non_media_at_rules_are_skipped() {
    // `@import ... ;` terminates at the semicolon.
    let s = Stylesheet::parse(r#"@import "x"; p{a:b}"#);
    assert_eq!(s.rules.len(), 1);
    // other conditional groups (`@supports`, `@layer`) skip their whole
    // (balanced) block — only `@media` is evaluated (bl-8ff4).
    let s = Stylesheet::parse("@supports (display:grid) { p{a:b} } div{c:d}");
    assert_eq!(s.rules.len(), 1);
    assert_eq!(s.rules[0].decls, vec![d("c", "d", false)]);
    // bare at-keyword at EOF parses to nothing.
    assert!(Stylesheet::parse("@charset").rules.is_empty());
}

#[test]
fn media_blocks_are_evaluated_against_the_fixed_viewport() {
    // A matching prelude contributes its rules to the cascade in place…
    let s = Stylesheet::parse("@media (min-width: 768px) { p{a:b} } div{c:d}");
    assert_eq!(s.rules.len(), 2);
    assert_eq!(s.rules[0].decls, vec![d("a", "b", false)]);
    // …a non-matching one contributes nothing.
    assert!(Stylesheet::parse("@media (min-width: 2000px) { p{a:b} }")
        .rules
        .is_empty());
    assert!(Stylesheet::parse("@media print { p{a:b} }")
        .rules
        .is_empty());
    // The at-keyword is case-insensitive.
    assert_eq!(Stylesheet::parse("@MEDIA screen { p{a:b} }").rules.len(), 1);
    // Statement form (no body) contributes nothing.
    assert!(Stylesheet::parse("@media screen;").rules.is_empty());
    // An unterminated body reads to end of sheet (as plain rules do); an
    // empty prelude means `all`.
    assert_eq!(Stylesheet::parse("@media { p{a:b}").rules.len(), 1);
}

#[test]
fn nested_media_blocks_multiply() {
    let s = Stylesheet::parse("@media screen { @media (max-width: 9999px) { p{a:b} } q{c:d} }");
    assert_eq!(s.rules.len(), 2);
    assert!(
        Stylesheet::parse("@media screen { @media print { p{a:b} } }")
            .rules
            .is_empty()
    );
}

#[test]
fn trailing_garbage_without_brace_ends_the_sheet() {
    let s = Stylesheet::parse("p{a:b} junk-no-brace");
    assert_eq!(s.rules.len(), 1);
}

#[test]
fn empty_selector_or_empty_body_skips_rule() {
    let s = Stylesheet::parse(" {a:b} p{c:d}");
    assert_eq!(s.rules.len(), 1);
    assert_eq!(s.rules[0].decls, vec![d("c", "d", false)]);
    let s = Stylesheet::parse("p{} q{a:b}");
    assert_eq!(s.rules.len(), 1);
}

#[test]
fn brace_inside_body_is_balanced() {
    let s = Stylesheet::parse("a{ b: f{g} } z{h:i}");
    assert_eq!(s.rules.len(), 2);
    assert_eq!(s.rules[0].decls, vec![d("b", "f{g}", false)]);
    assert_eq!(s.rules[1].decls, vec![d("h", "i", false)]);
}

#[test]
fn unterminated_body_is_still_read() {
    let s = Stylesheet::parse("a{b:c");
    assert_eq!(s.rules.len(), 1);
    assert_eq!(s.rules[0].decls, vec![d("b", "c", false)]);
}

#[test]
fn important_is_detected_and_case_insensitive() {
    assert_eq!(
        parse_decls("color:red!important"),
        vec![d("color", "red", true)]
    );
    assert_eq!(
        parse_decls("color: RED !IMPORTANT"),
        vec![d("color", "RED", true)]
    );
    // a space inside `! important` is not the keyword — kept literally.
    assert_eq!(
        parse_decls("color:red ! important"),
        vec![d("color", "red ! important", false)]
    );
    // value that is *only* !important collapses to nothing.
    assert!(parse_decls("x:!important").is_empty());
}

#[test]
fn malformed_declarations_drop() {
    assert!(parse_decls(":val").is_empty());
    assert!(parse_decls("name:").is_empty());
    assert!(parse_decls("novalue").is_empty());
    assert!(parse_decls("").is_empty());
    assert_eq!(
        parse_decls("a:b; c:d ;;"),
        vec![d("a", "b", false), d("c", "d", false)]
    );
}

#[test]
fn colon_inside_quotes_is_not_the_separator() {
    // top-level colon detection skips quoted regions.
    let decls = parse_decls(r#""sec:tion": red"#);
    assert_eq!(decls, vec![d(r#""sec:tion""#, "red", false)]);
    // a colon that only appears inside quotes leaves no separator.
    assert!(parse_decls(r#"" : ""#).is_empty());
}

/// A literal is opaque to declaration splitting even when it contains an
/// escaped delimiter: the `;` and `:` inside it are not separators.
#[test]
fn escaped_delimiter_keeps_a_literal_opaque_to_splitting() {
    assert_eq!(
        parse_decls(r#"content:"a\";b:c";display:none"#),
        vec![
            d("content", r#""a\";b:c""#, false),
            d("display", "none", false)
        ]
    );
}
