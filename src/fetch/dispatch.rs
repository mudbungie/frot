//! One request, hop by hop: the redirect loop every fetch rides, with the hop
//! budget, the cookie jar and the per-hop header derivation.
//!
//! A request is a GET unless it carries a [`ScriptBody`] — the one value that
//! makes it a POST, which only the JS subfetch path can construct
//! (`docs/design/challenge.md` §2). Nothing in this module names a verb.

use std::time::Duration;

use super::{
    decode, fetch_file, header_value, request, validate_url, FetchError, FetchResult, Intent,
    SharedJar, Transport, MAX_BODY_BYTES,
};
use crate::envelope::kinds;
use crate::js::ScriptBody;

/// One request as a call site describes it. `initiator` is the caller-`-H`
/// origin anchor; `referrer` is the URL that caused the request — `None` for
/// the caller's own navigation, the document for a subresource or for a hop the
/// document itself directed (`src/refresh.rs`).
pub(crate) struct Request<'a> {
    pub url: &'a str,
    pub intent: Intent,
    pub initiator: &'a str,
    pub referrer: Option<&'a str>,
    pub timeout: Duration,
    /// The page script's own request body: present, the request is the POST the
    /// script issued; absent — every Rust-side caller — it is a GET.
    pub body: Option<&'a ScriptBody>,
}

/// Spend one request from a hop budget; an empty budget is the one
/// [`kinds::FETCH_REDIRECT`] exhaustion, whatever asked for the hop.
fn spend(hops: &mut usize) -> Result<(), FetchError> {
    *hops = hops
        .checked_sub(1)
        .ok_or_else(|| FetchError::new(kinds::FETCH_REDIRECT, "too many redirects"))?;
    Ok(())
}

/// Issue `req` over the session's shared `transport`, deriving the
/// protocol-correct request metadata for each hop (`request::derive_headers`)
/// and running each exchange under `req.timeout`. Follows redirects while `hops`
/// lasts — every request spends one, so a caller that issues several dispatches
/// against one budget (a navigation: [`Navigation`]) bounds the whole chain —
/// **recomputing** `Sec-Fetch-Site`/`Referer` and the same-origin caller scope
/// for the new target (so cross-origin credentials, `Authorization` included,
/// never leak). `file://` reads take the read path and ignore the pool.
///
/// A redirected POST follows the browser rule: `307`/`308` re-send the body to
/// the new target, every other redirect continues as a GET without it.
pub(crate) fn dispatch(
    transport: &Transport,
    jar: &SharedJar,
    caller: &[(String, String)],
    req: &Request,
    hops: &mut usize,
) -> Result<FetchResult, FetchError> {
    let (intent, initiator) = (req.intent, req.initiator);
    let parsed = validate_url(req.url)?;
    spend(hops)?;
    if parsed.scheme() == "file" {
        return fetch_file(&parsed, intent);
    }
    // The *parsed* URL's serialization, not the caller's spelling: `final`,
    // every hop's request target, the cookie/referrer decisions, and anything
    // resolved against the page all read `current`, so it must be the one
    // canonical form. A redirect hop already serializes a `url::Url` (see
    // [`redirect_target`]), so without this a landing reached directly and the
    // same landing reached through a 302 would disagree about their own URL
    // (bl-2832). The caller's byte-for-byte spelling stays in `url.requested`.
    let mut current = parsed.to_string();
    // The referrer source: a subresource references its document across every
    // hop; a navigation redirect adopts the pre-redirect hop as the new source.
    let mut referrer = req.referrer.map(str::to_string);
    let mut body = req.body;
    loop {
        // The jar's applicable Cookie for this hop; `apply_caller` still layers a
        // caller `-H Cookie` over it as a single same-origin line (bl-6dad).
        let cookie = jar
            .lock()
            .unwrap()
            .header_for(&current, referrer.as_deref(), intent);
        let headers = request::derive_headers(
            intent,
            &current,
            initiator,
            referrer.as_deref(),
            caller,
            cookie.as_deref(),
            body,
        );
        let resp = transport.send(&current, &headers, body, MAX_BODY_BYTES, req.timeout)?;
        // Store this hop's Set-Cookie before following the redirect, so a
        // redirect-set cookie rides the landing request (acceptance #1).
        jar.lock().unwrap().store(&resp.headers, &current, intent);
        let Some(next) = redirect_target(resp.status, &resp.headers, &current)? else {
            let content_type = header_value(&resp.headers, "content-type");
            let bytes = decode::inflate(&resp.headers, resp.body)?;
            let (body, charset) = decode::body(intent, &bytes, content_type.as_deref());
            return Ok(FetchResult {
                final_url: current,
                status: Some(resp.status),
                headers: resp.headers,
                body,
                charset,
            });
        };
        spend(hops)?;
        body = body.filter(|_| matches!(resp.status, 307 | 308));
        if matches!(intent, Intent::Navigation) {
            referrer = Some(current.clone());
        }
        current = next;
    }
}

/// The absolute redirect target for a response, or `None` when it is terminal.
/// A 3xx with a resolvable `Location` redirects; a 3xx without one, or an
/// unresolvable `Location`, is an error rather than a silent stop.
pub(super) fn redirect_target(
    status: u16,
    headers: &[(String, String)],
    base: &str,
) -> Result<Option<String>, FetchError> {
    if !matches!(status, 301 | 302 | 303 | 307 | 308) {
        return Ok(None);
    }
    let location = header_value(headers, "location")
        .ok_or_else(|| FetchError::new(kinds::FETCH_REDIRECT, "redirect without Location"))?;
    let next = url::Url::parse(base)
        .and_then(|b| b.join(&location))
        .map_err(|e| FetchError::new(kinds::FETCH_REDIRECT, format!("bad Location: {e}")))?;
    Ok(Some(next.to_string()))
}
