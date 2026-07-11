//! Once-then-frozen subfetch cache (`docs/design/js.md` §6) — the JS-visible
//! network layer shared by `fetch`, `XMLHttpRequest`, and external `<script
//! src>`. One syscall (`super::syscall::net`) and the script runner
//! (`super::run_external`) both ride this cache; the prelude shapes `fetch`/XHR
//! over the syscall.
//!
//! The policy mirrors the stylesheet subfetch in `run.rs`: GET only (implicit —
//! the cache only ever calls [`crate::fetch::fetch`], which GETs), each absolute
//! URL fetched at most once and its response frozen for the call, `-H` headers
//! ride only same-origin, remote→local reads refused, and a `SUBFETCH_MAX` cap.
//! No live network after a URL first resolves; nothing persists past the call
//! (the cache dies with the [`super::Session`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use url::Url;

use crate::fetch::{self, FetchResult};

/// §6 / §13 OQ-1 default cap: requests per call. A constant, not a flag — the
/// tests dial it down through [`Subfetch::with_cap`], the same posture the engine
/// uses for the budget (`with_limits`).
pub const SUBFETCH_MAX: u32 = 16;

/// A frozen response, served for the call's lifetime once a URL resolves (§6).
/// `ok` is the fetch-spec 2xx range (a `file://` read, statusless, is ok); it is
/// the single source of "did this resource load" for `Response.ok`, XHR, and the
/// external-script run/skip decision.
#[derive(Debug, Clone)]
pub struct Frozen {
    pub ok: bool,
    pub status: u16,
    pub url: String,
    pub body: String,
    pub headers: Vec<(String, String)>,
}

/// A subfetch result: a frozen response, or the reason it was refused (policy) or
/// failed (transport). Both surface to JS the same way — `fetch` rejects, XHR
/// errors, an external `<script>` is skipped-and-counted (§4.2).
pub enum Outcome {
    Got(Frozen),
    Failed(String),
}

/// The per-call cache, shared (interior mutability) between the net syscall and
/// the Rust script runner, both of which fetch through it.
pub type SharedSubfetch = Rc<RefCell<Subfetch>>;

/// The once-then-frozen cache (§6), keyed by resolved absolute URL.
pub struct Subfetch {
    base: String,
    headers: Vec<(String, String)>,
    cache: HashMap<String, Frozen>,
    count: u32,
    max: u32,
}

impl Subfetch {
    /// A cache anchored at `base` (the final page URL) with the caller's
    /// `headers`, at the shipping [`SUBFETCH_MAX`] cap.
    pub fn new(base: &str, headers: Vec<(String, String)>) -> Self {
        Self::with_cap(base, headers, SUBFETCH_MAX)
    }

    /// [`Subfetch::new`] with an explicit cap for the tests. `base` is stored
    /// unparsed and validated per resolve, so a bogus page URL (which the
    /// `location` shim also tolerates) fails every fetch rather than panicking.
    pub fn with_cap(base: &str, headers: Vec<(String, String)>, max: u32) -> Self {
        Subfetch {
            base: base.to_string(),
            headers,
            cache: HashMap::new(),
            count: 0,
            max,
        }
    }

    /// Fetch `spec` (resolved against the page URL) once, frozen (§6). A cache
    /// hit re-serves the frozen response without touching the network; a miss
    /// spends one of the `max` requests, and past the cap every new URL is
    /// refused (counted by the caller). Same-origin requests carry the `-H`
    /// headers; cross-origin ones do not (credentials never leak).
    pub fn get(&mut self, spec: &str) -> Outcome {
        let url = match self.resolve(spec) {
            Ok(u) => u,
            Err(reason) => return Outcome::Failed(reason),
        };
        if let Some(frozen) = self.cache.get(&url) {
            return Outcome::Got(frozen.clone());
        }
        if self.count >= self.max {
            return Outcome::Failed(format!("subfetch cap reached ({} requests)", self.max));
        }
        self.count += 1;
        let headers: &[(String, String)] = if fetch::same_origin(&url, &self.base) {
            &self.headers
        } else {
            &[]
        };
        match fetch::fetch(&url, headers) {
            Ok(r) => {
                let frozen = freeze(r);
                self.cache.insert(url, frozen.clone());
                Outcome::Got(frozen)
            }
            Err(e) => Outcome::Failed(e.message),
        }
    }

    /// Resolve `spec` against the page URL and enforce the remote→local block: a
    /// `file:` target from an http(s) page is refused (§6), as with stylesheets.
    /// A bogus page URL (never a real final URL, but the shim tolerates it)
    /// fails here rather than panicking.
    fn resolve(&self, spec: &str) -> Result<String, String> {
        let base = Url::parse(&self.base).map_err(|e| e.to_string())?;
        let u = base.join(spec).map_err(|e| e.to_string())?;
        if u.scheme() == "file" && base.scheme() != "file" {
            return Err(format!("refused remote\u{2192}local fetch: {u}"));
        }
        Ok(u.to_string())
    }
}

/// Freeze a transport result into the JS-visible shape. A statusless `file://`
/// read counts as ok; an http status is ok in the 2xx range (fetch-spec `ok`).
fn freeze(r: FetchResult) -> Frozen {
    Frozen {
        ok: r.status.is_none_or(|s| (200..300).contains(&s)),
        status: r.status.unwrap_or(0),
        url: r.final_url,
        body: r.body,
        headers: r.headers,
    }
}

#[cfg(test)]
mod tests;
