//! CSS string escapes: scanning ([`Quoting`]) and decoding ([`unquote`]).

use super::*;

/// A hex escape denotes the code point it names — the icon-font case that
/// motivated this (`content:"\e609"` is one private-use character, not the
/// four literal characters `e609`). Case of the digits is irrelevant.
#[test]
fn hex_escape_decodes_to_its_code_point() {
    assert_eq!(unquote(r#""\e609""#), "\u{e609}");
    assert_eq!(unquote(r#""\E609""#), "\u{e609}");
    assert_eq!(unquote(r#""\2014""#), "—");
}

/// One optional whitespace unit terminates a hex escape and is swallowed;
/// `\r\n` counts as that one unit. A digit run stops at six digits, and at the
/// end of the literal.
#[test]
fn hex_escape_swallows_one_terminating_whitespace_unit() {
    assert_eq!(unquote(r#""\41 B""#), "AB");
    assert_eq!(unquote("\"\\41\r\nB\""), "AB");
    assert_eq!(unquote("\"\\41\rB\""), "AB");
    assert_eq!(unquote("\"\\41\tB\""), "AB");
    // Six digits max: the seventh character is literal text, not a digit.
    assert_eq!(unquote(r#""\0000411""#), "A1");
    // A digit run ending at the closing delimiter has no whitespace to eat.
    assert_eq!(unquote(r#""\41""#), "A");
}

/// Null, surrogate, and out-of-range values become U+FFFD rather than
/// vanishing or aborting the string.
#[test]
fn out_of_range_hex_escapes_become_the_replacement_character() {
    assert_eq!(unquote(r#""\0""#), "\u{fffd}");
    assert_eq!(unquote(r#""\d800""#), "\u{fffd}");
    assert_eq!(unquote(r#""\110000""#), "\u{fffd}");
}

/// A `\` before any non-hex, non-newline character denotes that character —
/// how an author escapes the delimiter or a literal backslash.
#[test]
fn simple_escape_denotes_the_escaped_character() {
    assert_eq!(unquote(r#""a\"b""#), "a\"b");
    assert_eq!(unquote(r#""a\\b""#), "a\\b");
    assert_eq!(unquote(r#""a\zb""#), "azb");
    assert_eq!(unquote(r#"'a\'b'"#), "a'b");
    // The other delimiter needs no escaping inside a literal.
    assert_eq!(unquote(r#""it's""#), "it's");
}

/// A `\` before a newline is a line continuation: it contributes nothing, and
/// neither does a `\` with nothing after it.
#[test]
fn escaped_newline_and_trailing_backslash_contribute_nothing() {
    assert_eq!(unquote("\"a\\\nb\""), "ab");
    assert_eq!(unquote("\"a\\\r\nb\""), "ab");
    assert_eq!(unquote("\"a\\\u{c}b\""), "ab");
    assert_eq!(unquote(r#""a\"#), "a");
}

/// An unterminated literal decodes to everything it has — the parser drops no
/// content just because the author forgot the closing quote.
#[test]
fn unterminated_literal_decodes_to_end_of_token() {
    assert_eq!(unquote(r#""a\41"#), "aA");
}

/// [`Quoting`] reports exactly the characters of a string literal, delimiters
/// included, and an escaped delimiter does not end it.
#[test]
fn quoting_tracks_literals_through_escaped_delimiters() {
    let mut q = Quoting::default();
    let inside: String = r#"a"b\"c"d"#
        .chars()
        .map(|c| if q.feed(c) { 'I' } else { 'O' })
        .collect();
    assert_eq!(inside, "OIIIIIIO");
    // A backslash outside a literal is structural, not an escape.
    let mut q = Quoting::default();
    assert!(!q.feed('\\'));
    assert!(q.feed('\''));
}
