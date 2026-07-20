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
