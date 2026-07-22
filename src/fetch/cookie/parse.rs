//! Set-Cookie / `document.cookie` attribute parsing (RFC 6265 §5.2), shared by
//! the wire path ([`super::CookieJar::store`]) and the JS write path
//! ([`super::CookieJar::write_script`]). A focused hand-rolled parser: the jar
//! needs only the attributes the design lists (Path, Domain, Secure, HttpOnly,
//! Max-Age, Expires, SameSite), so a full cookie crate's transitive tree and
//! static-musl weight buy nothing here (the dep checkpoint, `identity.md` §6).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use url::Url;

use super::{domain_match, Cookie, SameSite};
use crate::fetch::profile::CivilDate;

/// Parse one `Set-Cookie` line relative to `request_url`. `script` is a
/// `document.cookie` write, which can never set HttpOnly. Returns `None` for a
/// nameless/`=`-less line or a `Domain` the request host may not set — the cases
/// a browser silently drops. `Max-Age`/`Expires` in the past yield a cookie the
/// jar drops on insert (the delete path); `created` is assigned there.
pub(super) fn set_cookie(
    line: &str,
    request_url: &Url,
    now: SystemTime,
    script: bool,
) -> Option<Cookie> {
    // The name=value pair is everything before the first ';'; the rest are
    // attributes. Splitting this way (not `split(';').next()`) keeps every branch
    // reachable — a cookie with no attributes takes the `None` arm.
    let (head, rest) = match line.split_once(';') {
        Some((h, r)) => (h, r),
        None => (line, ""),
    };
    let (name, value) = head.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    let host = request_url.host_str()?;
    let mut c = Cookie {
        name: name.to_string(),
        value: value.trim().to_string(),
        domain: host.to_string(),
        host_only: true,
        path: default_path(request_url.path()),
        secure: false,
        http_only: false,
        same_site: SameSite::Lax,
        expires: None,
        created: 0,
    };
    let (mut max_age, mut expires) = (None, None);
    for attr in rest.split(';') {
        let (k, v) = match attr.split_once('=') {
            Some((k, v)) => (k.trim().to_ascii_lowercase(), v.trim().to_string()),
            None => (attr.trim().to_ascii_lowercase(), String::new()),
        };
        match k.as_str() {
            "path" if v.starts_with('/') => c.path = v,
            "domain" if !v.is_empty() => set_domain(&mut c, host, &v)?,
            "secure" => c.secure = true,
            "httponly" => c.http_only = true,
            "samesite" => c.same_site = same_site(&v),
            "max-age" => max_age = v.parse::<i64>().ok(),
            "expires" => expires = parse_date(&v),
            _ => {}
        }
    }
    c.http_only = c.http_only && !script;
    // Max-Age wins over Expires (RFC 6265 §5.3); non-positive is an instant delete.
    c.expires = match max_age {
        Some(s) => Some(if s > 0 {
            now + Duration::from_secs(s as u64)
        } else {
            UNIX_EPOCH
        }),
        None => expires,
    };
    Some(c)
}

/// Apply a `Domain=` attribute: strip a leading dot, lowercase, and require the
/// request host to domain-match it (else the cookie is rejected). Sets the cookie
/// as non-host-only so it matches subdomains.
fn set_domain(c: &mut Cookie, host: &str, raw: &str) -> Option<()> {
    let domain = raw.trim_start_matches('.').to_ascii_lowercase();
    if !domain_match(host, &domain, false) {
        return None;
    }
    c.domain = domain;
    c.host_only = false;
    Some(())
}

/// The `SameSite` value; anything but `strict`/`none` (including absent) is `Lax`.
fn same_site(v: &str) -> SameSite {
    match v.to_ascii_lowercase().as_str() {
        "strict" => SameSite::Strict,
        "none" => SameSite::None,
        _ => SameSite::Lax,
    }
}

/// RFC 6265 §5.1.4 default-path: the request-uri path up to (not including) the
/// rightmost `/`, or `/` if that is the leading slash. The `url` crate always
/// yields a path starting with `/`, so the rightmost slash always exists.
fn default_path(path: &str) -> String {
    let cut = path.rfind('/').expect("request-uri path starts with '/'");
    if cut == 0 {
        "/".to_string()
    } else {
        path[..cut].to_string()
    }
}

/// Parse an `Expires` HTTP-date (`Wdy, DD Mon YYYY HH:MM:SS GMT`) to an absolute
/// time, reusing the profile's civil→epoch algorithm (SSOT). An unparseable date
/// is `None`, which a browser treats as a session cookie (attribute ignored) — a
/// declared residual for the relaxed RFC 6265 §5.1.1 tokenizer and 2-digit years.
fn parse_date(s: &str) -> Option<SystemTime> {
    let after_comma = s.split_once(',').map_or(s, |(_, r)| r);
    let toks: Vec<&str> = after_comma.split_whitespace().collect();
    // `day mon year HH:MM:SS [GMT]` — a trailing zone is tolerated and ignored.
    let [day, mon, year, time, ..] = toks[..] else {
        return None;
    };
    let hms: Vec<&str> = time.split(':').collect();
    let [h, min, sec] = hms[..] else {
        return None;
    };
    let days = CivilDate {
        year: year.parse().ok()?,
        month: month_num(mon)?,
        day: day.parse().ok()?,
    }
    .days_since_epoch();
    let total = days * 86_400
        + h.parse::<i64>().ok()? * 3_600
        + min.parse::<i64>().ok()? * 60
        + sec.parse::<i64>().ok()?;
    u64::try_from(total)
        .ok()
        .map(|s| UNIX_EPOCH + Duration::from_secs(s))
}

/// Month abbreviation → 1..=12, or `None`.
fn month_num(m: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let m = m.to_ascii_lowercase();
    MONTHS.iter().position(|x| *x == m).map(|i| i as i64 + 1)
}

#[cfg(test)]
mod tests;
