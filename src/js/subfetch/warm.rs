//! The concurrent warm of the §6 cache (`bl-08f6`) — the preload-scanner pass
//! over the initial external scripts.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::fetch::{fetch_many, Intent, TIMEOUT_SECS};

use super::{data, freeze, Subfetch};

impl Subfetch {
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
}
