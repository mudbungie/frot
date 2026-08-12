use super::*;

fn run(html: &str, base: &str) -> Vec<Value> {
    match forms(&Document::parse(html), base) {
        Value::Array(a) => a,
        _ => panic!("forms did not return an array"),
    }
}

#[test]
fn no_form_returns_empty() {
    assert!(run("<p>no forms</p>", "https://example.com/").is_empty());
}

#[test]
fn empty_form_picks_up_action_method_enctype_defaults() {
    let r = run("<form></form>", "https://example.com/page");
    let f = &r[0];
    assert_eq!(f["action"], "https://example.com/page");
    assert_eq!(f["method"], "get");
    assert_eq!(f["enctype"], "application/x-www-form-urlencoded");
    assert_eq!(f["fields"], json!([]));
}

#[test]
fn method_lowercased() {
    let r = run("<form method='POST'></form>", "https://example.com/");
    assert_eq!(r[0]["method"], "post");
}

#[test]
fn action_resolved_against_page_url() {
    let r = run("<form action='/submit'></form>", "https://example.com/page");
    assert_eq!(r[0]["action"], "https://example.com/submit");
}

#[test]
fn base_href_overrides_for_form_action() {
    let r = run(
        "<head><base href='https://api.example.com/'></head><body><form action='do'></form></body>",
        "https://example.com/",
    );
    assert_eq!(r[0]["action"], "https://api.example.com/do");
}

#[test]
fn root_relative_base_href_still_yields_an_absolute_action() {
    // bl-409e: a raw `/` base left the action unresolved. One authority now
    // resolves it against the page first, so forms match links and meta.
    let r = run(
        "<head><base href='/'></head><body><form action='search'></form></body>",
        "https://example.com/docs/intro",
    );
    assert_eq!(r[0]["action"], "https://example.com/search");
}

#[test]
fn explicit_enctype_kept() {
    let r = run(
        "<form enctype='multipart/form-data'></form>",
        "https://example.com/",
    );
    assert_eq!(r[0]["enctype"], "multipart/form-data");
}

#[test]
fn input_field_extracted_with_type_and_value() {
    let r = run(
        "<form><input name='q' type='text' value='hi'></form>",
        "https://example.com/",
    );
    let f = &r[0]["fields"][0];
    assert_eq!(f["tag"], "input");
    assert_eq!(f["name"], "q");
    assert_eq!(f["type"], "text");
    assert_eq!(f["value"], "hi");
    assert_eq!(f["required"], false);
}

#[test]
fn required_flag_recognized() {
    let r = run(
        "<form><input name='q' required></form>",
        "https://example.com/",
    );
    assert_eq!(r[0]["fields"][0]["required"], true);
}

#[test]
fn textarea_value_comes_from_text_content() {
    let r = run(
        "<form><textarea name='bio'>about me</textarea></form>",
        "https://example.com/",
    );
    let f = &r[0]["fields"][0];
    assert_eq!(f["tag"], "textarea");
    assert_eq!(f["value"], "about me");
    assert_eq!(f["type"], Value::Null);
}

#[test]
fn button_has_type_attribute() {
    let r = run(
        "<form><button type='submit' name='go' value='1'>Go</button></form>",
        "https://example.com/",
    );
    let f = &r[0]["fields"][0];
    assert_eq!(f["tag"], "button");
    assert_eq!(f["type"], "submit");
    assert_eq!(f["value"], "1");
}

#[test]
fn select_with_options_extracted() {
    let r = run(
        "<form><select name='c'><option value='a'>A</option><option>B</option></select></form>",
        "https://example.com/",
    );
    let f = &r[0]["fields"][0];
    assert_eq!(f["tag"], "select");
    assert_eq!(f["options"][0]["value"], "a");
    assert_eq!(f["options"][0]["label"], "A");
    assert_eq!(f["options"][1]["value"], "B");
    assert_eq!(f["options"][1]["label"], "B");
}

#[test]
fn multiple_forms_returned_in_order() {
    let r = run(
        "<form action='/1'></form><form action='/2'></form>",
        "https://example.com/",
    );
    assert_eq!(r.len(), 2);
    assert_eq!(r[0]["action"], "https://example.com/1");
    assert_eq!(r[1]["action"], "https://example.com/2");
}

#[test]
fn input_without_name_or_value_still_present() {
    let r = run("<form><input></form>", "https://example.com/");
    let f = &r[0]["fields"][0];
    assert_eq!(f["name"], Value::Null);
    assert_eq!(f["value"], Value::Null);
}

#[test]
fn empty_base_falls_back_to_page_url() {
    let r = run(
        "<head><base href=''></head><body><form action='/x'></form></body>",
        "https://example.com/",
    );
    assert_eq!(r[0]["action"], "https://example.com/x");
}

#[test]
fn unparseable_page_url_leaves_action_as_is() {
    let r = run("<form action='/x'></form>", "not a url");
    assert_eq!(r[0]["action"], "/x");
}
