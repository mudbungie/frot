//! Once-then-frozen subfetch cache (`docs/design/js.md` §6) — the JS-visible
//! network layer shared by `fetch`, `XMLHttpRequest`, external `<script src>`,
//! and the ES-module loader. One syscall (`super::syscall::net`) and the script
//! runner (`super::run_external`) both ride this cache; the prelude shapes
//! `fetch`/XHR over the syscall.
//!
//! The policy mirrors the stylesheet subfetch in `run.rs`: GET only (implicit —
//! the cache only ever calls [`FetchSession::subresource`], which GETs), each absolute
//! URL fetched at most once and its response frozen for the call, `-H` headers
//! ride only same-origin, remote→local reads refused. Bounds are one per real
//! resource (§6, bl-c7e9): *time* — the §5 wall-clock [`Deadline`] is consulted
//! before every network dispatch (the engine interrupt cannot fire inside a
//! blocking host fetch chain, so the seam enforces the same clock) — and
//! *memory* — a pooled [`SUBFETCH_BYTES`] response-body budget across the call.
//! There is deliberately no request-count cap: a count measures no resource and
//! starved code-split apps while time and memory stood idle. No live network
//! after a URL first resolves; nothing persists past the call (the cache dies
//! with the [`super::Session`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use url::Url;

use crate::fetch::{FetchResult, FetchSession, Intent, TIMEOUT_SECS};

use super::engine::Deadline;

/// §6 pooled response-byte budget per call. The frozen cache is the network-side
/// analogue of the engine heap and takes the same allowance as
/// `engine::JS_MEM_LIMIT`. A constant, not a flag — the tests dial it down
/// through [`Subfetch::with_budget`], the same posture the engine uses for the
/// time budget (`with_limits`).
pub const SUBFETCH_BYTES: usize = 64 * 1024 * 1024;

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

/// The once-then-frozen cache (§6), keyed by resolved absolute URL. Its network
/// dispatch rides the invocation's shared [`FetchSession`] (bl-5191): every
/// subfetch reuses the one connection pool, and the `-H` same-origin scoping is
/// the session's — this layer owns only the §6 freezing, deadline, and byte
/// pool on top of it.
pub struct Subfetch {
    session: FetchSession,
    base: String,
    cache: HashMap<String, Frozen>,
    deadline: Deadline,
    spent: usize,
    budget: usize,
}

impl Subfetch {
    /// A cache anchored at `base` (the final page URL) over the shared
    /// `session`, dispatching only inside the engine's armed `deadline` window
    /// and the shipping [`SUBFETCH_BYTES`] pool.
    pub fn new(session: FetchSession, base: &str, deadline: Deadline) -> Self {
        Self::with_budget(session, base, deadline, SUBFETCH_BYTES)
    }

    /// [`Subfetch::new`] with an explicit byte budget for the tests. `base` is
    /// stored unparsed and validated per resolve, so a bogus page URL (which the
    /// `location` shim also tolerates) fails every fetch rather than panicking.
    pub fn with_budget(
        session: FetchSession,
        base: &str,
        deadline: Deadline,
        budget: usize,
    ) -> Self {
        Subfetch {
            session,
            base: base.to_string(),
            cache: HashMap::new(),
            deadline,
            spent: 0,
            budget,
        }
    }

    /// Fetch `spec` (resolved against the page URL) once, frozen (§6). A cache
    /// hit re-serves the frozen response without touching the network; a miss
    /// dispatches through the shared session only while the §5 deadline stands
    /// and the byte pool has room — past either, every new URL is refused
    /// (counted by the caller). Overshoot is bounded to one in-flight response.
    /// `intent` (fetch/XHR, external script, or module) rides to the session as
    /// the bl-20ec per-destination seam; the same-origin `-H` scoping is the
    /// session's.
    pub fn get(&mut self, spec: &str, intent: Intent) -> Outcome {
        let url = match self.resolve(spec) {
            Ok(u) => u,
            Err(reason) => return Outcome::Failed(reason),
        };
        if let Some(frozen) = self.cache.get(&url) {
            return Outcome::Got(frozen.clone());
        }
        if self.deadline.expired() {
            return Outcome::Failed("subfetch refused: run budget exhausted".to_string());
        }
        if self.spent >= self.budget {
            return Outcome::Failed(format!(
                "subfetch byte budget exhausted ({} bytes)",
                self.budget
            ));
        }
        let timeout = Duration::from_secs(TIMEOUT_SECS);
        match self.session.subresource(&url, &self.base, intent, timeout) {
            Ok(r) => {
                let frozen = freeze(r);
                self.spent += frozen.body.len();
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
