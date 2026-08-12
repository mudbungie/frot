//! Once-then-frozen subfetch cache (`docs/design/js.md` §6) — the JS-visible
//! network layer shared by `fetch`, `XMLHttpRequest`, external `<script src>`,
//! and the ES-module loader. One syscall (`super::syscall::net`) and the script
//! runner (`super::run_external`) both ride this cache; the prelude shapes
//! `fetch`/XHR over the syscall.
//!
//! The policy mirrors the stylesheet subfetch in `run.rs`: GET only (implicit —
//! the cache only ever calls [`FetchSession::subresource`], which GETs), each URL
//! fetched at most once and its response frozen for the call, `-H` headers ride
//! only same-origin, remote→local reads refused. Bounds are one per real
//! resource, each in its own unit (§6, bl-c7e9/bl-8dc0/bl-79dc): *time* — every
//! dispatch is bracketed on the run's clock and charged to the §5 [`NetBudget`],
//! which is consulted before the next one (the engine interrupt cannot fire
//! inside a blocking host fetch chain, and anyway spends CPU, which a blocked
//! socket does not burn); a refusal is *recorded* ([`Subfetch::refused`]) as the
//! fact the run driver reads to name §10 `stopped: "network"` — true whenever
//! emitted, because only the wire can spend that budget — and *memory*: a pooled
//! [`SUBFETCH_BYTES`] response-body budget across the call. A `data:` URL skips
//! the network half of that: it carries its own bytes ([`data`]), so it is
//! decoded locally and charged only to the byte pool. There is deliberately no
//! request-count cap: a count measures no resource and starved code-split apps
//! while time and memory stood idle. Nothing persists past the call.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use url::Url;

use crate::fetch::{fetch_many, FetchResult, FetchSession, Intent, TIMEOUT_SECS};

use super::engine::NetBudget;

mod data;

/// §6 pooled response-byte budget per call. The frozen cache is the network-side
/// analogue of the engine heap and takes the same allowance as
/// `engine::JS_MEM_LIMIT`. A constant, not a flag — the tests dial it down through
/// [`Subfetch::with_budget`], the posture the engine uses for the time bounds.
pub const SUBFETCH_BYTES: usize = 64 * 1024 * 1024;

/// A frozen response, served for the call's lifetime once a URL resolves (§6).
/// `ok` is the fetch-spec 2xx range (a `file://` read, statusless, is ok) and the
/// single source of "did this resource load" for `Response.ok`, XHR, and the
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
/// failed (transport). Both surface to JS alike — `fetch` rejects, XHR errors, an
/// external `<script>` is skipped-and-counted (§4.2).
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
/// the session's — this layer owns only the §6 freezing, the network budget's
/// metering, and the byte pool on top of it.
pub struct Subfetch {
    session: FetchSession,
    base: String,
    cache: HashMap<String, Frozen>,
    net: NetBudget,
    spent: usize,
    budget: usize,
    refused: bool,
    timings: Vec<ResourceTiming>,
}

/// A real measured subfetch (`bl-e707`), exposed to JS as a
/// `PerformanceResourceTiming` (js.md §8). Only a request that actually hit the
/// network is recorded — a cache hit re-serves a frozen response and a refusal
/// never dispatches, so neither is a new measurement (nor a network charge). frot
/// measures `start`/`duration` off the one §5 clock; every phase it does not
/// (DNS, TCP, TLS) is spec-legal `0` in the prelude, never a fabricated number.
struct ResourceTiming {
    name: String,
    start_ns: u64,
    dur_ns: u64,
}

impl Subfetch {
    /// A cache anchored at `base` (the final page URL) over the shared
    /// `session`, dispatching only while the run's `net` budget and the shipping
    /// [`SUBFETCH_BYTES`] pool have room.
    pub fn new(session: FetchSession, base: &str, net: NetBudget) -> Self {
        Self::with_budget(session, base, net, SUBFETCH_BYTES)
    }

    /// [`Subfetch::new`] with an explicit byte budget for the tests. `base` is
    /// stored unparsed and validated per resolve, so a bogus page URL (which the
    /// `location` shim tolerates) fails every fetch rather than panicking.
    pub fn with_budget(session: FetchSession, base: &str, net: NetBudget, budget: usize) -> Self {
        Subfetch {
            session,
            base: base.to_string(),
            cache: HashMap::new(),
            net,
            spent: 0,
            budget,
            refused: false,
            timings: Vec::new(),
        }
    }

    /// Whether this cache refused a dispatch because the §5 network budget was
    /// spent (§6). The run driver reads it once at conclusion to name
    /// `stopped: "network"` and clear `settled` — the truncation is an event, so
    /// it is recorded when it happens rather than inferred from a clock later.
    pub fn refused(&self) -> bool {
        self.refused
    }

    /// The real resource-timing measurements recorded so far (`bl-e707`), as
    /// `(name, start_ms, duration_ms)` — the shape `performance.getEntriesByType`
    /// builds each entry from. Times are ms on the run's one clock (the
    /// `__frot_now` origin), so they cohere with `performance.now()`.
    pub fn timings(&self) -> Vec<(String, f64, f64)> {
        self.timings
            .iter()
            .map(|t| {
                (
                    t.name.clone(),
                    t.start_ns as f64 / 1e6,
                    t.dur_ns as f64 / 1e6,
                )
            })
            .collect()
    }

    /// Fetch `spec` (resolved against the page URL) once, frozen (§6). A cache
    /// hit re-serves the frozen response without touching the network; a
    /// `data:` URL is decoded from its own bytes ([`Subfetch::local`]); a miss
    /// dispatches through the shared session only while the §5 network budget and
    /// the byte pool have room — past either, every new URL is refused (counted
    /// by the caller). Overshoot is bounded to one in-flight response. `intent`
    /// (fetch/XHR, external script, or module) rides to the session as the
    /// bl-20ec per-destination seam; the `-H` scoping is the session's.
    pub fn get(&mut self, spec: &str, intent: Intent) -> Outcome {
        let url = match self.resolve(spec) {
            Ok(u) => u,
            Err(reason) => return Outcome::Failed(reason),
        };
        if let Some(frozen) = self.cache.get(&url) {
            return Outcome::Got(frozen.clone());
        }
        if let Some(payload) = data::payload(&url) {
            return self.local(&url, payload);
        }
        if self.net.spent_out() {
            // The recorded fact behind §10 `stopped: "network"`: the run spent
            // its whole allowance on the wire and this seam turned the next
            // request away.
            self.refused = true;
            return Outcome::Failed("subfetch refused: network budget exhausted".to_string());
        }
        if self.spent >= self.budget {
            return Outcome::Failed(self.exhausted());
        }
        let timeout = Duration::from_secs(TIMEOUT_SECS);
        // Bracket the real dispatch on the run's one clock: its elapsed is at
        // once the §5 network charge (`bl-79dc`) and the resource entry's *actual*
        // duration (`bl-e707`). A failed dispatch — a timeout is the slowest
        // origin there is — is charged exactly as a served one.
        let start_ns = self.net.elapsed_nanos();
        let (result, dur_ns) = self
            .net
            .charge(|| self.session.subresource(&url, &self.base, intent, timeout));
        match result {
            Ok(r) => {
                let frozen = freeze(r);
                self.spent += frozen.body.len();
                self.timings.push(ResourceTiming {
                    name: url.clone(),
                    start_ns,
                    dur_ns,
                });
                self.cache.insert(url, frozen.clone());
                Outcome::Got(frozen)
            }
            Err(e) => Outcome::Failed(e.message),
        }
    }

    /// Serve a `data:` URL from its own bytes (js.md §4.1). The payload is the
    /// response, so nothing is dispatched: the §5 network budget — a bound on
    /// *network* time — is neither consulted nor charged, and a page's inline scripts run in
    /// document order however late the wall clock is. Memory still bounds it: the
    /// decoded body charges the pooled byte budget exactly as a fetched body
    /// does. Deliberately not cached — the cache key would be the payload itself,
    /// so freezing would store the bytes twice to make a pure decode repeatable.
    fn local(&mut self, url: &str, payload: &str) -> Outcome {
        if self.spent >= self.budget {
            return Outcome::Failed(self.exhausted());
        }
        match data::decode(url, payload) {
            Ok(frozen) => {
                self.spent += frozen.body.len();
                Outcome::Got(frozen)
            }
            Err(reason) => Outcome::Failed(reason),
        }
    }

    /// The one message for a spent byte pool, shared by both paths.
    fn exhausted(&self) -> String {
        format!("subfetch byte budget exhausted ({} bytes)", self.budget)
    }

    /// Warm the cache concurrently with `specs` — the initial external scripts
    /// discovered after parse (bl-08f6) — so the source-ordered queue that runs
    /// next finds each already frozen. Each spec is resolved and the not-yet-cached
    /// absolute URLs are fetched in parallel through the shared [`fetch_many`]
    /// primitive, under this cache's *same* network budget and the byte pool's
    /// remainder, then folded into the cache in one single-threaded pass that
    /// charges `spent` exactly as a serial [`get`](Self::get) would. The wave's
    /// own elapsed is charged to the network budget too — it is wire time however
    /// many sockets shared it. Freezing here means a later `get` for a warmed URL
    /// is a plain cache hit — served even past the budget, since no network is
    /// left to gate. Failures are dropped: execution re-attempts and counts them
    /// (§4.2), never a warm-time error.
    pub fn warm(&mut self, specs: &[(String, Intent)]) {
        let mut seen = HashSet::new();
        let mut reqs = Vec::new();
        for (spec, intent) in specs {
            if let Ok(url) = self.resolve(spec) {
                // A `data:` URL has no round trip to hide, so there is nothing to
                // warm: it decodes when the queue reaches it.
                if data::payload(&url).is_none()
                    && !self.cache.contains_key(&url)
                    && seen.insert(url.clone())
                {
                    reqs.push((url, *intent));
                }
            }
        }
        let left = self.net.left().min(Duration::from_secs(TIMEOUT_SECS));
        let budget = self.budget.saturating_sub(self.spent);
        let (wave, _) = self.net.charge(|| {
            fetch_many(
                &self.session,
                &reqs,
                &self.base,
                Instant::now() + left,
                budget,
            )
        });
        for (i, r) in wave {
            let frozen = freeze(r);
            self.spent += frozen.body.len();
            self.cache.insert(reqs[i].0.clone(), frozen);
        }
    }

    /// Resolve `spec` against the page URL and enforce the remote→local block: a
    /// `file:` target from an http(s) page is refused (§6), as with stylesheets.
    /// A bogus page URL fails here rather than panicking.
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
