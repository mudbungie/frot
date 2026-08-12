//! `content:` value tokenization and string building.

use super::*;
use crate::dom::Attr;

fn el(attrs: &[(&str, &str)]) -> Element {
    Element {
        name: "p".to_string(),
        attrs: attrs
            .iter()
            .map(|(n, v)| Attr {
                name: n.to_string(),
                value: v.to_string(),
            })
            .collect(),
    }
}

/// A run of whitespace between tokens flushes an already-empty buffer: the
/// second separator must not emit a phantom empty token.
#[test]
fn repeated_separators_emit_no_empty_tokens() {
    assert_eq!(tokens(r#""a"  "b""#), vec![r#""a""#, r#""b""#]);
    assert_eq!(tokens(r#" "a" "#), vec![r#""a""#]);
    assert!(tokens("   ").is_empty());
}

/// The generated string concatenates literals and `attr()` across those runs,
/// so double spacing in the declaration is invisible in the output.
#[test]
fn double_spaced_value_concatenates() {
    let p = el(&[("data-x", "X")]);
    assert_eq!(string(r#""a"  attr(data-x)  "b""#, &p), "aXb");
    assert_eq!(string(r#""a" attr(data-x) "b""#, &p), "aXb");
}

/// A literal is one token however it is spelled: an escaped delimiter does not
/// end it, and whitespace inside it is not a token separator.
#[test]
fn escaped_delimiter_does_not_split_a_literal() {
    assert_eq!(tokens(r#""a\" b" "c""#), vec![r#""a\" b""#, r#""c""#]);
    assert_eq!(string(r#""a\" b" "c""#, &el(&[])), "a\" bc");
}

/// Escapes are decoded before the value is concatenated, so a hex escape
/// reaches text/AX/layout as its character and neighbouring tokens are
/// unaffected.
#[test]
fn escapes_decode_within_concatenated_tokens() {
    let p = el(&[("data-x", "X")]);
    assert_eq!(string(r#""\e609" attr(data-x) "\2014""#, &p), "\u{e609}X—");
    // A backslash outside a literal is not an escape: `attr()` is unaffected.
    assert_eq!(string(r#""\\" attr(data-x)"#, &p), "\\X");
}
