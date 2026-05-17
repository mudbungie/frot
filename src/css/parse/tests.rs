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
fn at_rules_are_skipped() {
    // `@import ... ;` terminates at the semicolon.
    let s = Stylesheet::parse(r#"@import "x"; p{a:b}"#);
    assert_eq!(s.rules.len(), 1);
    // `@media { ... }` skips its whole (balanced) block.
    let s = Stylesheet::parse("@media screen { p{a:b} } div{c:d}");
    assert_eq!(s.rules.len(), 1);
    assert_eq!(s.rules[0].decls, vec![d("c", "d", false)]);
    // unterminated block and bare at-keyword at EOF parse to nothing.
    assert!(Stylesheet::parse("@media { p{a:b}").rules.is_empty());
    assert!(Stylesheet::parse("@charset").rules.is_empty());
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
