use super::imports;

#[test]
fn string_and_url_forms_are_reported_in_source_order() {
    let css = "@charset \"utf-8\";\n/* c */ @import \"a.css\";\n@import 'b.css';\n\
               @IMPORT URL(c.css);\n@import url( \"d.css\" );\n@import url('e.css');\np{}";
    assert_eq!(imports(css), ["a.css", "b.css", "c.css", "d.css", "e.css"]);
}

#[test]
fn a_semicolon_or_paren_inside_the_target_does_not_end_the_rule() {
    assert_eq!(
        imports("@import url('https://f.example/css2?family=A:wght@400;700');p{}"),
        ["https://f.example/css2?family=A:wght@400;700"]
    );
    assert_eq!(imports("@import url(a;b.css);"), ["a;b.css"]);
    assert_eq!(imports("@import \"a).css\" screen;"), ["a).css"]);
    assert_eq!(imports("@import url(\"a).css\");"), ["a).css"]);
    assert_eq!(imports("@import \"a\\\"b.css\";"), ["a\"b.css"]);
}

#[test]
fn the_media_list_decides_whether_an_import_applies() {
    let css = "@import 'a.css' screen;@import 'b.css' print;\
               @import url(c.css) screen and (min-width: 600px);\
               @import url(d.css) (max-width: 600px);@import 'e.css' print, all;";
    assert_eq!(imports(css), ["a.css", "c.css", "e.css"]);
}

#[test]
fn layer_and_supports_qualifiers_are_not_applied() {
    assert!(imports("@import 'a.css' layer;").is_empty());
    assert!(imports("@import url(a.css) layer(base);").is_empty());
    assert!(imports("@import 'a.css' supports(display: grid);").is_empty());
}

#[test]
fn imports_count_only_before_the_first_other_rule() {
    assert_eq!(imports("@layer a, b;@import 'a.css';"), ["a.css"]);
    assert!(imports("p{} @import 'a.css';").is_empty());
    assert!(imports("@media screen{p{}} @import 'a.css';").is_empty());
    assert!(imports("@font-face{} @import 'a.css';").is_empty());
    assert!(imports("@namespace svg url(x); @import 'a.css';").is_empty());
    assert!(imports("@layer a{p{}} @import 'a.css';").is_empty());
    assert!(imports("@import 'a.css'{}").is_empty());
}

#[test]
fn malformed_and_empty_inputs_report_nothing() {
    assert!(imports("").is_empty());
    assert!(imports("  ").is_empty());
    assert!(imports("@import a.css;").is_empty());
    assert!(imports("@import url(a.css;").is_empty());
    assert!(imports("@import é;").is_empty());
    // An unterminated string reads to the end of the rule.
    assert_eq!(imports("@import 'a.css"), ["a.css"]);
}
