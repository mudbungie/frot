use super::*;

fn run(html: &str, base: &str) -> Vec<Value> {
    match links(&Document::parse(html), base) {
        Value::Array(a) => a,
        _ => panic!("links did not return an array"),
    }
}

fn href_of(v: &Value) -> &str {
    v.get("href").and_then(|x| x.as_str()).unwrap()
}

#[test]
fn no_links_returns_empty_array() {
    let r = run("<p>nothing here</p>", "https://example.com/");
    assert!(r.is_empty());
}

#[test]
fn absolute_anchor_kept_verbatim() {
    let r = run(
        "<a href='https://other.com/x'>x</a>",
        "https://example.com/",
    );
    assert_eq!(href_of(&r[0]), "https://other.com/x");
}

#[test]
fn relative_anchor_resolved_against_page_url() {
    let r = run("<a href='/about'>about</a>", "https://example.com/contact");
    assert_eq!(href_of(&r[0]), "https://example.com/about");
}

#[test]
fn base_href_overrides_page_url() {
    let r = run(
        "<head><base href='https://cdn.example.com/'></head><body><a href='x'>x</a></body>",
        "https://example.com/",
    );
    assert_eq!(href_of(&r[0]), "https://cdn.example.com/x");
}

#[test]
fn root_relative_base_href_still_yields_absolute_links() {
    // bl-409e, the angular.dev shape: `<base href="/">` is not itself a URL, so
    // a raw base left every href unresolved. Resolved against the page it is the
    // origin root, and the icon link comes out absolute — as `document.baseURI`
    // and the element's `href` report in Chrome at the same final URL.
    let r = run(
        "<head><base href='/'>\
         <link rel='apple-touch-icon' href='/assets/icons/apple-touch-icon.png'></head>\
         <body><a href='guide'>guide</a></body>",
        "https://example.com/docs/intro",
    );
    assert_eq!(
        href_of(&r[0]),
        "https://example.com/assets/icons/apple-touch-icon.png"
    );
    assert_eq!(href_of(&r[1]), "https://example.com/guide");
}

#[test]
fn path_relative_base_href_resolves_against_the_page_directory() {
    let r = run(
        "<head><base href='build/'></head><body><a href='x'>x</a></body>",
        "https://example.com/docs/intro",
    );
    assert_eq!(href_of(&r[0]), "https://example.com/docs/build/x");
}

#[test]
fn scheme_relative_base_href_takes_the_page_scheme() {
    let r = run(
        "<head><base href='//cdn.example.com/a/'></head><body><a href='x'>x</a></body>",
        "https://example.com/",
    );
    assert_eq!(href_of(&r[0]), "https://cdn.example.com/a/x");
}

#[test]
fn invalid_base_href_falls_back_to_page_url() {
    let r = run(
        "<head><base href='http://'></head><body><a href='/y'>y</a></body>",
        "https://example.com/",
    );
    assert_eq!(href_of(&r[0]), "https://example.com/y");
}

#[test]
fn only_the_first_base_href_counts() {
    let r = run(
        "<head><base href='/first/'><base href='/second/'></head>\
         <body><a href='x'>x</a></body>",
        "https://example.com/",
    );
    assert_eq!(href_of(&r[0]), "https://example.com/first/x");
}

#[test]
fn empty_base_href_falls_back_to_page_url() {
    let r = run(
        "<head><base href=''></head><body><a href='/y'>y</a></body>",
        "https://example.com/",
    );
    assert_eq!(href_of(&r[0]), "https://example.com/y");
}

#[test]
fn anchor_text_normalized() {
    let r = run(
        "<a href='/x'>  hello\n   world  </a>",
        "https://example.com/",
    );
    assert_eq!(r[0]["text"], Value::String("hello world".into()));
}

#[test]
fn link_element_has_null_text() {
    let r = run(
        "<link href='/style.css' rel='stylesheet'>",
        "https://example.com/",
    );
    assert_eq!(r[0]["text"], Value::Null);
    assert_eq!(r[0]["kind"], "link");
}

#[test]
fn area_treated_as_anchor() {
    let r = run(
        "<map><area href='/zone' alt='Zone'></map>",
        "https://example.com/",
    );
    assert_eq!(r[0]["kind"], "area");
    assert_eq!(href_of(&r[0]), "https://example.com/zone");
}

#[test]
fn rel_split_into_token_list() {
    let r = run(
        "<a href='/x' rel='noopener noreferrer external'>x</a>",
        "https://example.com/",
    );
    assert_eq!(r[0]["rel"], json!(["noopener", "noreferrer", "external"]));
}

#[test]
fn missing_rel_is_empty_array() {
    let r = run("<a href='/x'>x</a>", "https://example.com/");
    assert_eq!(r[0]["rel"], json!([]));
}

#[test]
fn anchor_without_href_is_omitted() {
    let r = run("<a>no href</a><a href='/y'>yes</a>", "https://example.com/");
    assert_eq!(r.len(), 1);
    assert_eq!(href_of(&r[0]), "https://example.com/y");
}

#[test]
fn unparseable_page_url_passes_href_through() {
    let r = run("<a href='not://a real:url'>x</a>", "not a url");
    assert_eq!(href_of(&r[0]), "not://a real:url");
}

#[test]
fn unresolvable_relative_href_kept_as_is() {
    let r = run("<a href='/x'>x</a>", "not a url");
    assert_eq!(href_of(&r[0]), "/x");
}

#[test]
fn base_without_href_falls_back_to_page_url() {
    let r = run(
        "<head><base target='_blank'></head><body><a href='/y'>y</a></body>",
        "https://example.com/",
    );
    assert_eq!(href_of(&r[0]), "https://example.com/y");
}

#[test]
fn multiple_anchors_returned_in_order() {
    let r = run(
        "<a href='/1'>1</a><a href='/2'>2</a>",
        "https://example.com/",
    );
    assert_eq!(r.len(), 2);
    assert_eq!(href_of(&r[0]), "https://example.com/1");
    assert_eq!(href_of(&r[1]), "https://example.com/2");
}
