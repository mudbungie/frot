use super::*;

#[test]
fn caller_headers_are_sent() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/")
        .match_header("x-api-key", "sekrit")
        .match_header("cookie", "a=b")
        .with_body("ok")
        .create();
    let headers = vec![
        ("X-Api-Key".to_string(), "sekrit".to_string()),
        ("Cookie".to_string(), "a=b".to_string()),
    ];
    let r = fetch(&server.url(), &headers).unwrap();
    assert_eq!(r.status, Some(200));
    m.assert();
}

#[test]
fn caller_user_agent_replaces_the_default() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/")
        .match_header("user-agent", "custom-agent/1")
        .with_body("ok")
        .create();
    let headers = vec![("User-Agent".to_string(), "custom-agent/1".to_string())];
    fetch(&server.url(), &headers).unwrap();
    m.assert();
}

#[test]
fn default_user_agent_applies_without_override() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/")
        .match_header("user-agent", USER_AGENT)
        .with_body("ok")
        .create();
    fetch(&server.url(), &[]).unwrap();
    m.assert();
}

#[test]
fn document_get_sends_the_firefox_navigation_set() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/")
        .match_header(
            "accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,\
             image/avif,image/webp,*/*;q=0.8",
        )
        .match_header("accept-language", "en-US,en;q=0.5")
        .match_header("upgrade-insecure-requests", "1")
        .match_header("sec-fetch-dest", "document")
        .match_header("sec-fetch-mode", "navigate")
        .match_header("sec-fetch-site", "none")
        .match_header("sec-fetch-user", "?1")
        .match_header("user-agent", USER_AGENT)
        .with_body("ok")
        .create();
    fetch_document(&server.url(), &[]).unwrap();
    m.assert();
}

#[test]
fn caller_accept_replaces_the_default_without_duplicating() {
    let mut server = mockito::Server::new();
    // Exact match on the override proves the default Accept was not also
    // sent (a duplicate would make the received value differ); the untouched
    // Accept-Language default must still ride alongside.
    let m = server
        .mock("GET", "/")
        .match_header("accept", "application/json")
        .match_header("accept-language", "en-US,en;q=0.5")
        .with_body("ok")
        .create();
    let headers = vec![("Accept".to_string(), "application/json".to_string())];
    fetch_document(&server.url(), &headers).unwrap();
    m.assert();
}

#[test]
fn plain_fetch_omits_the_navigation_set() {
    let mut server = mockito::Server::new();
    // Subresource subfetches use `fetch`, not `fetch_document`: no nav headers.
    let m = server
        .mock("GET", "/")
        .match_header("sec-fetch-mode", mockito::Matcher::Missing)
        .match_header("accept-language", mockito::Matcher::Missing)
        .with_body("ok")
        .create();
    fetch(&server.url(), &[]).unwrap();
    m.assert();
}

#[test]
fn same_origin_compares_scheme_host_and_port() {
    assert!(same_origin("http://a.example/x", "http://a.example/y?z"));
    assert!(same_origin("https://a.example:443/", "https://a.example/p"));
    assert!(!same_origin("http://a.example/", "https://a.example/"));
    assert!(!same_origin("http://a.example/", "http://b.example/"));
    assert!(!same_origin("http://a.example:81/", "http://a.example/"));
    assert!(!same_origin("not a url", "http://a.example/"));
}
