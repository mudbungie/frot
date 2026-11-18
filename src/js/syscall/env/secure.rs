//! Whether the document is a **secure context** — the one fact behind
//! `window.isSecureContext` and behind every `[SecureContext]` member the
//! prelude publishes (`crypto.subtle`, `crypto.randomUUID`).
//!
//! Every row is READ off Firefox 153.0esr (`bl-b0f8`, identity.md §3.19), not
//! taken from the Secure Contexts spec: `https:` and `file:` are secure; `http:`
//! is secure only on a loopback host (`127.0.0.0/8`, `[::1]`, `localhost`,
//! `*.localhost`); `about:blank`, `data:` and any other `http:` host are not.

use url::{Host, Url};

/// True iff a real Firefox loading `raw` reports `isSecureContext === true`.
/// A URL that does not parse is no context at all, hence not a secure one.
pub fn is_secure_context(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    let loopback = match url.host() {
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        Some(Host::Domain(name)) => name == "localhost" || name.ends_with(".localhost"),
        None => false,
    };
    matches!(url.scheme(), "https" | "file") || (url.scheme() == "http" && loopback)
}

#[cfg(test)]
mod tests {
    use super::is_secure_context;

    #[test]
    fn the_measured_rows() {
        for secure in [
            "https://example.com/",
            "file:///tmp/page.html",
            "http://127.0.0.1:8080/",
            "http://127.0.0.2/",
            "http://[::1]:8080/",
            "http://localhost:8080/",
            "http://foo.localhost/",
        ] {
            assert!(is_secure_context(secure), "{secure}");
        }
        for insecure in [
            "http://example.com/",
            "http://[2001:db8::1]/",
            "http://notlocalhost/",
            "about:blank",
            "data:text/html,<p>x",
            "not a url",
        ] {
            assert!(!is_secure_context(insecure), "{insecure}");
        }
    }
}
