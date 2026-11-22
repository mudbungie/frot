//! The CSS-gather phase: collect external `<link rel=stylesheet>` sheets and
//! the sheets they (or a `<style>` block) `@import`, concurrently and under one
//! aggregate wall-clock budget.
//!
//! Two bounds, one per real resource (`docs/design/css.md` §1a, mirroring
//! `docs/design/js.md` §6's "one bound per resource, none per request count"):
//!
//! - **Time — [`GATHER_BUDGET_MS`], for the whole phase.** Not per request:
//!   a per-request ceiling multiplied by an attacker/host-controlled sheet
//!   count is not a bound. The deadline is the single authority, and each
//!   request's timeout is *derived* from what the budget has left
//!   ([`fetch::fetch_within`]), so a host that never answers costs the phase
//!   its remaining budget and nothing more — there is no in-flight overshoot.
//! - **Concurrency — `POOL_PER_HOST` simultaneous requests** (bl-08f6's shared
//!   [`crate::fetch::fetch_many`]). This bounds sockets and OS threads, which
//!   are real resources; it is deliberately *not* a cap on how many sheets are
//!   fetched. Every sheet is still attempted, in document order, until time runs
//!   out — the count of sheets prices no resource (the argument that retired
//!   `SUBFETCH_MAX`, js.md §6).
//!
//! *Which* sheets are gathered is a cascade question, not a fetch one: a
//! `<link>`'s `media` attribute decides whether its sheet participates, and it
//! is evaluated by the one media authority (`src/css/media.rs`, via
//! [`media_applies`]) — so a print-only sheet never reaches the screen cascade.
//!
//! Tripping the budget is not a run failure. CSS is best-effort here as it
//! always has been: whatever arrived applies, in source order, and the
//! cascade proceeds.

use std::time::{Duration, Instant};

use crate::dom::{Document, NodeKind, WalkEvent};
use crate::fetch::{fetch_many, FetchSession, Intent};
use url::Url;

/// Aggregate wall-clock ceiling for the entire gather phase.
///
/// A backstop, not the mechanism: concurrency is what makes the common case
/// fast (the pathological real page, linear.app's 68 Next.js chunks, gathers
/// well inside this once six requests fly at once), and this bound exists so
/// that one slow or dead host cannot set the floor for the run. It is sized
/// from both ends — comfortably above a full wave of legitimately slow sheets
/// (a cold cross-origin CDN handshake plus transfer, ~1–2 s), and far below
/// the per-request `fetch::TIMEOUT_SECS` ceiling it replaces as the phase's
/// effective limit, so `--css` can no longer cost 15 s × N. A constant, not a
/// flag, on the same severability posture as the 1280 px viewport and
/// `EXEC_CPU_MS`: nothing about a caller's page makes a different number
/// right, and a flag here would be a tunable for a behaviour that should just
/// be correct.
const GATHER_BUDGET_MS: u64 = 5_000;

/// Fetch every external stylesheet the document links, best-effort: CSS is
/// non-critical, so a failed or unfinished fetch is skipped rather than
/// failing the run. Returns the bodies that arrived, in document order —
/// source order is cascade order, so it must survive concurrency.
///
/// Every sheet rides `session`: the workers share its one connection pool (one
/// bounded pool — not one isolated agent per sheet), and an unchanged sheet
/// already gathered this invocation is re-served from the session cache without
/// a second fetch. The `-H` scoping and credentials rule live in the session.
pub(crate) fn external_css(doc: &Document, base: &str, session: &FetchSession) -> Vec<String> {
    let budget = Duration::from_millis(GATHER_BUDGET_MS);
    gather_within(&external_hrefs(doc, base), base, session, budget)
}

/// One sheet of the import graph: a `<link>`/`<style>` root (`parent` is
/// `None`) or a sheet some other sheet `@import`s.
struct Sheet {
    url: String,
    parent: Option<usize>,
    /// The fetched text; `None` until (and unless) the fetch succeeds.
    body: Option<String>,
    /// The sheets this one imports, in source order.
    imports: Vec<usize>,
}

/// [`external_css`]'s gather with the phase budget supplied by the caller —
/// the seam the budget-trip tests dial down, so the bound is exercised
/// without waiting on wall-clock (js.md's `Session::with_bounds` pattern).
///
/// The bounded concurrent fetch is the shared [`fetch_many`] primitive (bl-08f6);
/// the cascade is source-ordered, so completion order is discarded and the
/// bodies are restored to href (document) order. CSS is time-bounded only — no
/// byte cap (`usize::MAX`): every sheet is attempted until the budget lapses.
///
/// `@import` rides the same path: each wave's arrivals name the next wave (one
/// wave per import level, so siblings still fly together), under the same
/// deadline, session and intent. An imported sheet's text is returned ahead of
/// its importer's — where the `@import` rule stands in the cascade.
pub(crate) fn gather_within(
    hrefs: &[String],
    base: &str,
    session: &FetchSession,
    budget: Duration,
) -> Vec<String> {
    let deadline = Instant::now() + budget;
    let local = page_is_file(base);
    let mut sheets: Vec<Sheet> = Vec::new();
    for h in hrefs {
        adopt(&mut sheets, None, h.clone());
    }
    let mut wave = 0..sheets.len();
    while !wave.is_empty() {
        let reqs: Vec<(String, Intent)> = sheets[wave.clone()]
            .iter()
            .map(|s| (s.url.clone(), Intent::Style))
            .collect();
        let next = sheets.len();
        let mut got = fetch_many(session, &reqs, base, deadline, usize::MAX);
        got.sort_by_key(|(i, _)| *i);
        for (i, r) in got {
            let id = wave.start + i;
            let at = Url::parse(&r.final_url).ok();
            for url in imported(&r.body, at.as_ref(), local) {
                if admissible(&sheets, id, &url) {
                    adopt(&mut sheets, Some(id), url);
                }
            }
            sheets[id].body = Some(r.body);
        }
        wave = next..sheets.len();
    }
    let mut out = Vec::new();
    for root in 0..hrefs.len() {
        flatten(&mut sheets, root, &mut out);
    }
    out
}

fn adopt(sheets: &mut Vec<Sheet>, parent: Option<usize>, url: String) {
    let id = sheets.len();
    if let Some(p) = parent {
        sheets[p].imports.push(id);
    }
    sheets.push(Sheet {
        url,
        parent,
        body: None,
        imports: Vec::new(),
    });
}

/// How many sheets deep an `@import` chain is followed. Real chains are one or
/// two links; the phase budget is the bound that prices anything, and this only
/// keeps an origin that mints a fresh import per response from spending all of
/// it.
const IMPORT_DEPTH: usize = 8;

/// Whether `importer` may import `url`: not when `url` is the importer itself
/// or one of the sheets that led to it (an import cycle contributes nothing new
/// — the sheet is already on its way into the cascade), and not past
/// [`IMPORT_DEPTH`].
fn admissible(sheets: &[Sheet], importer: usize, url: &str) -> bool {
    let mut depth = 1;
    let mut at = Some(importer);
    while let Some(id) = at {
        if sheets[id].url == url || depth > IMPORT_DEPTH {
            return false;
        }
        depth += 1;
        at = sheets[id].parent;
    }
    true
}

/// Append `id`'s imports (recursively, in source order) and then its own text:
/// the order the cascade would have read them had each `@import` been replaced
/// by the sheet it names. A sheet that never arrived contributes nothing.
fn flatten(sheets: &mut [Sheet], id: usize, out: &mut Vec<String>) {
    for child in std::mem::take(&mut sheets[id].imports) {
        flatten(sheets, child, out);
    }
    out.extend(sheets[id].body.take());
}

fn page_is_file(page_url: &str) -> bool {
    Url::parse(page_url).is_ok_and(|p| p.scheme() == "file")
}

/// `href` resolved against `base`, if a sheet there may be gathered: empty
/// hrefs and ones that fail to resolve are dropped, and a `file:` sheet is kept
/// only when the *page* is `file:` (`local`) — remote content must never cause
/// local reads, whether it is a `<base>` the page controls or an `@import` in a
/// sheet it serves.
fn resolve(base: Option<&Url>, href: &str, local: bool) -> Option<String> {
    let u = base?.join(Some(href).filter(|h| !h.is_empty())?).ok()?;
    (u.scheme() != "file" || local).then(|| u.to_string())
}

/// The sheets `css` (a sheet at `base`) `@import`s into the screen cascade —
/// [`crate::css::imports`] decides which rules count — as gatherable URLs.
fn imported(css: &str, base: Option<&Url>, local: bool) -> Vec<String> {
    crate::css::imports(css)
        .iter()
        .filter_map(|href| resolve(base, href, local))
        .collect()
}

/// Absolute URLs of the sheets the document itself brings into the screen
/// cascade — every `<link rel="stylesheet">` and every `@import` of a `<style>`
/// block, in document order — resolved against the document base URL
/// ([`crate::base`]): the same authority the views resolve through, so a
/// `<base href>` moves the sheets a browser would fetch and the ones frot
/// fetches together. Non-stylesheet links and elements whose `media` does not
/// apply ([`media_applies`]) are dropped; [`resolve`] drops the rest.
pub(crate) fn external_hrefs(doc: &Document, page_url: &str) -> Vec<String> {
    let base_url = crate::base::base_url(doc, page_url);
    let local = page_is_file(page_url);
    let mut out = Vec::new();
    doc.walk(None, &mut |ev, e| {
        let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) else {
            return;
        };
        if !media_applies(el) {
            return;
        }
        let is_sheet = el.name == "link"
            && el.attr("rel").is_some_and(|r| {
                r.split_whitespace()
                    .any(|t| t.eq_ignore_ascii_case("stylesheet"))
            });
        if is_sheet {
            let href = el.attr("href").unwrap_or_default();
            out.extend(resolve(base_url.as_ref(), href, local));
        } else if el.name == "style" {
            out.extend(imported(&doc.text_content(id), base_url.as_ref(), local));
        }
    });
    out
}

/// Whether a `<link>`'s (or a `<style>`'s, for its imports) `media` attribute
/// admits its sheet to the screen cascade. The attribute *is* a media-query list — the same grammar as an
/// `@media` prelude — so it is evaluated by the same single authority
/// ([`crate::css::media`], which JS `matchMedia` also delegates to): no second
/// evaluator, no way for `<link media=print>` and `@media print` to disagree.
/// An absent attribute is `all`, and a malformed list is `not all` (media.rs's
/// conservative rule), so it never participates.
///
/// A non-matching sheet is dropped here, before the fetch: the contract is
/// cascade behaviour, and a sheet that cannot affect styles has no reason to
/// cost a request. Browser fetch priority is not modelled.
fn media_applies(el: &crate::dom::Element) -> bool {
    el.attr("media").is_none_or(crate::css::media::matches)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod import_tests;
