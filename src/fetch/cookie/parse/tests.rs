//! Set-Cookie attribute parsing: the name/value split, each attribute, the
//! default-path derivation, and the `Expires` date parser (every failure edge).

use std::time::{SystemTime, UNIX_EPOCH};

use url::Url;

use super::*;

fn url(s: &str) -> Url {
    Url::parse(s).unwrap()
}

/// Parse a wire `Set-Cookie` (not a script write) relative to `req`.
fn wire(line: &str, req: &str) -> Option<Cookie> {
    set_cookie(line, &url(req), SystemTime::now(), false)
}

#[test]
fn a_nameless_or_equals_less_line_is_ignored() {
    assert!(wire("justname", "https://x.com/").is_none());
    assert!(wire("=v", "https://x.com/").is_none());
}

#[test]
fn a_hostless_request_url_parses_nothing() {
    // `set_cookie` needs a host to scope the cookie to.
    assert!(set_cookie("a=1", &url("file:///x"), SystemTime::now(), false).is_none());
}

#[test]
fn name_value_is_trimmed_and_defaults_are_host_only_lax_root() {
    let c = wire("  a = 1 ", "https://x.com/p").unwrap();
    assert_eq!((c.name.as_str(), c.value.as_str()), ("a", "1"));
    assert!(c.host_only && !c.secure && !c.http_only);
    assert_eq!(c.same_site, SameSite::Lax);
    assert_eq!(c.domain, "x.com");
    assert_eq!(c.path, "/"); // default-path of "/p" is "/"
}

#[test]
fn default_path_is_the_directory_of_the_request() {
    assert_eq!(wire("a=1", "https://x.com/a/b").unwrap().path, "/a");
    assert_eq!(wire("a=1", "https://x.com/a").unwrap().path, "/");
    assert_eq!(wire("a=1", "https://x.com/").unwrap().path, "/");
}

#[test]
fn path_attribute_needs_a_leading_slash_else_default() {
    assert_eq!(
        wire("a=1; Path=/x/y", "https://x.com/").unwrap().path,
        "/x/y"
    );
    // A relative Path attribute is ignored (default-path stands).
    assert_eq!(
        wire("a=1; Path=rel", "https://x.com/a/b").unwrap().path,
        "/a"
    );
}

#[test]
fn domain_attribute_strips_the_dot_and_an_empty_one_is_ignored() {
    let c = wire("a=1; Domain=.X.com", "https://www.x.com/").unwrap();
    assert_eq!((c.domain.as_str(), c.host_only), ("x.com", false));
    // Empty Domain= keeps the host-only default.
    let e = wire("a=1; Domain=", "https://www.x.com/").unwrap();
    assert!(e.host_only);
    // A Domain the host cannot set rejects the whole cookie.
    assert!(wire("a=1; Domain=foo.com", "https://www.x.com/").is_none());
}

#[test]
fn boolean_and_samesite_attributes_and_unknowns() {
    let c = wire("a=1; Secure; HttpOnly; Weird=x", "https://x.com/").unwrap();
    assert!(c.secure && c.http_only);
    for (v, want) in [
        ("Strict", SameSite::Strict),
        ("none", SameSite::None),
        ("lax", SameSite::Lax),
        ("bogus", SameSite::Lax),
    ] {
        let c = wire(&format!("a=1; SameSite={v}"), "https://x.com/").unwrap();
        assert_eq!(c.same_site, want);
    }
}

#[test]
fn a_script_write_can_never_set_http_only() {
    let c = set_cookie(
        "a=1; HttpOnly",
        &url("https://x.com/"),
        SystemTime::now(),
        true,
    )
    .unwrap();
    assert!(!c.http_only);
}

#[test]
fn max_age_expiry_and_precedence_over_expires() {
    let now = UNIX_EPOCH;
    // Positive Max-Age → a future expiry.
    let f = set_cookie("a=1; Max-Age=60", &url("https://x.com/"), now, false).unwrap();
    assert!(f.expires.unwrap() > now);
    // Non-positive Max-Age → epoch (an instant delete on insert).
    let z = set_cookie("a=1; Max-Age=0", &url("https://x.com/"), now, false).unwrap();
    assert_eq!(z.expires, Some(UNIX_EPOCH));
    // A non-numeric Max-Age is ignored → falls back to Expires (here absent).
    let bad = set_cookie("a=1; Max-Age=x", &url("https://x.com/"), now, false).unwrap();
    assert_eq!(bad.expires, None);
    // Max-Age wins over Expires when both are present.
    let both = set_cookie(
        "a=1; Max-Age=60; Expires=Wed, 09 Jun 1990 10:18:14 GMT",
        &url("https://x.com/"),
        now,
        false,
    )
    .unwrap();
    assert!(both.expires.unwrap() > now);
}

#[test]
fn expires_parses_a_valid_date_and_ignores_a_bad_one() {
    let c = wire(
        "a=1; Expires=Wed, 09 Jun 2099 10:18:14 GMT",
        "https://x.com/",
    )
    .unwrap();
    assert!(c.expires.unwrap() > SystemTime::now());
    // Unparseable → session cookie (attribute ignored).
    assert_eq!(
        wire("a=1; Expires=nonsense", "https://x.com/")
            .unwrap()
            .expires,
        None
    );
}

#[test]
fn parse_date_handles_every_field_and_failure() {
    // Comma-less prefix is tolerated.
    assert!(parse_date("09 Jun 2099 10:18:14 GMT").is_some());
    // A date before the epoch is unrepresentable → None.
    assert!(parse_date("Wed, 09 Jun 1900 00:00:00 GMT").is_none());
    for bad in [
        "Wed,",                      // too few tokens (none)
        "Wed, 09 Jun 2099",          // too few tokens (no time)
        "Wed, 09 Jun 2099 10:18",    // time is not H:M:S
        "Wed, xx Jun 2099 10:18:14", // bad day
        "Wed, 09 Xxx 2099 10:18:14", // bad month
        "Wed, 09 Jun yyyy 10:18:14", // bad year
        "Wed, 09 Jun 2099 aa:18:14", // bad hour
        "Wed, 09 Jun 2099 10:mm:14", // bad minute
        "Wed, 09 Jun 2099 10:18:zz", // bad second
    ] {
        assert!(parse_date(bad).is_none(), "{bad} should not parse");
    }
}

#[test]
fn month_num_maps_names_and_rejects_junk() {
    assert_eq!(month_num("JAN"), Some(1));
    assert_eq!(month_num("dec"), Some(12));
    assert_eq!(month_num("xxx"), None);
}
