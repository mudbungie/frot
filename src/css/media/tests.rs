use super::matches;

#[test]
fn empty_list_means_all_and_matches() {
    assert!(matches(""));
    assert!(matches("   "));
}

#[test]
fn media_types_screen_and_all_match_print_does_not() {
    assert!(matches("screen"));
    assert!(matches("all"));
    assert!(matches("SCREEN")); // case-insensitive
    assert!(!matches("print"));
    // Unknown media type: conservative not-match…
    assert!(!matches("speech"));
    // …even under `not` (spec: unknown is malformed, malformed is `not all`).
    assert!(!matches("not speech"));
}

#[test]
fn only_is_a_noop_and_not_negates() {
    assert!(matches("only screen"));
    assert!(matches("not print"));
    assert!(!matches("not screen"));
    assert!(!matches("not (min-width: 768px)"));
}

#[test]
fn width_features_compare_against_the_1280px_viewport() {
    assert!(matches("(min-width: 768px)"));
    assert!(matches("(min-width: 1280px)"));
    assert!(!matches("(min-width: 1281px)"));
    assert!(matches("(max-width: 1280px)"));
    assert!(!matches("(max-width: 600px)"));
    assert!(matches("(width: 1280px)"));
    assert!(!matches("(width: 1279px)"));
}

#[test]
fn height_features_compare_against_the_720px_viewport() {
    assert!(matches("(min-height: 720px)"));
    assert!(!matches("(min-height: 721px)"));
    assert!(matches("(max-height: 720px)"));
    assert!(!matches("(max-height: 719px)"));
    assert!(matches("(height: 720px)"));
    assert!(!matches("(height: 100px)"));
}

#[test]
fn em_and_rem_convert_at_16px_per_unit() {
    // 80em == 1280px == the viewport width (js.md §7).
    assert!(matches("(min-width: 80em)"));
    assert!(!matches("(min-width: 81em)"));
    assert!(matches("(min-width: 40rem)"));
    assert!(matches("(min-width: 79.9em)")); // fractional values parse
    assert!(matches("(min-width: 768PX)")); // units are case-insensitive
}

#[test]
fn and_requires_every_condition() {
    assert!(matches("screen and (min-width: 768px)"));
    assert!(matches("(min-width: 768px) and (max-width: 9999px)"));
    assert!(!matches("screen and (min-width: 9999px)"));
    assert!(!matches("print and (min-width: 768px)"));
    // An unknown condition poisons the whole query even after a false type.
    assert!(!matches("print and (orientation: landscape)"));
}

#[test]
fn comma_is_or_over_the_query_list() {
    assert!(matches("print, (min-width: 768px)"));
    assert!(matches("(min-width: 9999px), screen"));
    assert!(!matches("print, (min-width: 9999px)"));
}

#[test]
fn unknown_features_units_and_values_never_match() {
    assert!(!matches("(orientation: landscape)")); // unknown feature, unitless value
    assert!(!matches("(min-device-width: 700px)")); // unknown feature, valid length
    assert!(!matches("(hover)")); // boolean form: no value to compare
    assert!(!matches("(min-width: 768)")); // missing unit
    assert!(!matches("(min-width: 50vw)")); // unknown unit
    assert!(!matches("(min-width: abcpx)")); // unparseable number
    assert!(!matches("(min-width: calc(700px))")); // nested parens lex, calc doesn't eval
}

#[test]
fn malformed_grammar_never_matches() {
    assert!(!matches("(min-width: 768px")); // unbalanced paren
    assert!(!matches("screen)")); // stray close paren
    assert!(!matches("screen (min-width: 768px)")); // missing `and`
    assert!(!matches("screen and")); // trailing `and`
    assert!(!matches("screen and and (min-width: 768px)")); // ident after `and`
    assert!(!matches("(min-width: 768px) or (min-width: 9999px)")); // `or` unsupported
    assert!(!matches("only")); // keyword with nothing after it
}
