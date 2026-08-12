use super::*;

fn run(html: &str, page_url: &str) -> Value {
    meta(&Document::parse(html), page_url)
}

#[test]
fn empty_document_returns_all_nulls() {
    let v = run("", "https://example.com/");
    assert_eq!(v["title"], Value::Null);
    assert_eq!(v["lang"], Value::Null);
    assert_eq!(v["charset"], Value::Null);
    assert_eq!(v["canonical"], Value::Null);
    assert_eq!(v["meta"], json!([]));
}

#[test]
fn title_extracted_and_trimmed() {
    let v = run(
        "<html><head><title>  Hello, World!  </title></head></html>",
        "https://example.com/",
    );
    assert_eq!(v["title"], "Hello, World!");
}

#[test]
fn html_lang_extracted() {
    let v = run("<html lang='en-US'></html>", "https://example.com/");
    assert_eq!(v["lang"], "en-US");
}

#[test]
fn meta_charset_lowercased() {
    let v = run(
        "<html><head><meta charset='UTF-8'></head></html>",
        "https://example.com/",
    );
    assert_eq!(v["charset"], "utf-8");
}

#[test]
fn meta_http_equiv_content_type_supplies_charset() {
    let v = run(
        "<html><head><meta http-equiv='Content-Type' content='text/html; charset=iso-8859-1'></head></html>",
        "https://example.com/",
    );
    assert_eq!(v["charset"], "iso-8859-1");
}

#[test]
fn meta_charset_wins_over_http_equiv() {
    let v = run(
        "<head><meta charset='utf-8'><meta http-equiv='Content-Type' content='text/html; charset=latin1'></head>",
        "https://example.com/",
    );
    assert_eq!(v["charset"], "utf-8");
}

#[test]
fn canonical_link_resolved_against_page_url() {
    let v = run(
        "<head><link rel='canonical' href='/canon'></head>",
        "https://example.com/page",
    );
    assert_eq!(v["canonical"], "https://example.com/canon");
}

#[test]
fn canonical_resolved_against_base_href() {
    let v = run(
        "<head><base href='https://cdn.example.com/'><link rel='canonical' href='/canon'></head>",
        "https://example.com/",
    );
    assert_eq!(v["canonical"], "https://cdn.example.com/canon");
}

#[test]
fn root_relative_base_href_still_yields_an_absolute_canonical() {
    // bl-409e: a raw `/` base left the canonical unresolved.
    let v = run(
        "<head><base href='/'><link rel='canonical' href='canon'></head>",
        "https://example.com/docs/intro",
    );
    assert_eq!(v["canonical"], "https://example.com/canon");
}

#[test]
fn canonical_kept_as_is_when_base_unparseable() {
    let v = run(
        "<head><link rel='canonical' href='/canon'></head>",
        "not a url",
    );
    assert_eq!(v["canonical"], "/canon");
}

#[test]
fn meta_with_name_and_content_collected() {
    let v = run(
        "<head><meta name='description' content='A page about things'></head>",
        "https://example.com/",
    );
    let entry = &v["meta"][0];
    assert_eq!(entry["name"], "description");
    assert_eq!(entry["content"], "A page about things");
}

#[test]
fn meta_with_property_collected_for_og() {
    let v = run(
        "<head><meta property='og:title' content='Hi'></head>",
        "https://example.com/",
    );
    let entry = &v["meta"][0];
    assert_eq!(entry["property"], "og:title");
    assert_eq!(entry["content"], "Hi");
}

#[test]
fn meta_with_http_equiv_collected() {
    let v = run(
        "<head><meta http-equiv='refresh' content='5'></head>",
        "https://example.com/",
    );
    let entry = &v["meta"][0];
    assert_eq!(entry["http-equiv"], "refresh");
    assert_eq!(entry["content"], "5");
}

#[test]
fn meta_without_content_skipped() {
    let v = run("<head><meta name='lonely'></head>", "https://example.com/");
    assert_eq!(v["meta"], json!([]));
}

#[test]
fn meta_without_classifier_skipped() {
    let v = run(
        "<head><meta content='orphan'></head>",
        "https://example.com/",
    );
    assert_eq!(v["meta"], json!([]));
}

#[test]
fn multiple_titles_first_wins() {
    let v = run(
        "<head><title>first</title><title>second</title></head>",
        "https://example.com/",
    );
    assert_eq!(v["title"], "first");
}

#[test]
fn extract_charset_handles_quoted_value() {
    assert_eq!(
        extract_charset("text/html; charset=\"utf-8\""),
        Some("utf-8")
    );
}

#[test]
fn extract_charset_returns_none_when_absent() {
    assert_eq!(extract_charset("text/html"), None);
}

#[test]
fn extract_charset_returns_none_when_value_empty() {
    assert_eq!(extract_charset("text/html; charset="), None);
}

#[test]
fn http_equiv_without_content_does_not_set_charset() {
    let v = run(
        "<head><meta http-equiv='Content-Type'></head>",
        "https://example.com/",
    );
    assert_eq!(v["charset"], Value::Null);
}

/// A second `<meta charset>` does not overwrite the first: charset is
/// first-wins, as a browser's encoding sniff is.
#[test]
fn first_meta_charset_wins_over_a_later_one() {
    let v = run(
        "<head><meta charset='UTF-8'><meta charset='iso-8859-1'></head>",
        "https://example.com/",
    );
    assert_eq!(v["charset"], "utf-8");
}

/// A `Content-Type` http-equiv whose content carries no `charset=` parameter
/// leaves charset unset — the header matches, the value simply has nothing.
#[test]
fn http_equiv_content_type_without_charset_param_sets_nothing() {
    let v = run(
        "<head><meta http-equiv='Content-Type' content='text/html'></head>",
        "https://example.com/",
    );
    assert_eq!(v["charset"], Value::Null);
}

#[test]
fn http_equiv_with_unrelated_content_does_not_set_charset() {
    let v = run(
        "<head><meta http-equiv='Refresh' content='5'></head>",
        "https://example.com/",
    );
    assert_eq!(v["charset"], Value::Null);
}

#[test]
fn empty_base_href_falls_back_to_page_url() {
    let v = run(
        "<head><base href=''><link rel='canonical' href='/canon'></head>",
        "https://example.com/",
    );
    assert_eq!(v["canonical"], "https://example.com/canon");
}

#[test]
fn link_without_canonical_rel_ignored() {
    let v = run(
        "<head><link rel='stylesheet' href='/style.css'></head>",
        "https://example.com/",
    );
    assert_eq!(v["canonical"], Value::Null);
}

#[test]
fn no_canonical_link_yields_null() {
    let v = run("<head></head>", "https://example.com/");
    assert_eq!(v["canonical"], Value::Null);
}

#[test]
fn canonical_rel_match_is_case_insensitive() {
    let v = run(
        "<head><link rel='Canonical' href='/c'></head>",
        "https://example.com/",
    );
    assert_eq!(v["canonical"], "https://example.com/c");
}
