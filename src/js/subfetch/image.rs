//! The `--images` wave (`bl-49f1`, js.md §7 "Images"): fetch the queued
//! `<img>` URLs in one concurrent wave and remember what each one is.

use std::time::{Duration, Instant};

use crate::fetch::{binary_bytes, fetch_many, header_value, Intent, TIMEOUT_SECS};
use crate::js::syscall::image_size;

use super::{data, Subfetch};

impl Subfetch {
    /// Fetch every image in `reqs` — `(resolved src, background)` — whose URL
    /// has no outcome yet, in one concurrent wave through [`fetch_many`] (the
    /// preload-warm primitive): each request times out at the §5 network
    /// budget's remainder, the wave's elapsed is charged once, and each body
    /// counts against the byte pool. Every URL then has an outcome — its size,
    /// or `None` for a failed, cut-off or unsizable one — which [`image`]
    /// re-serves; nothing is fetched twice. A wave the budget cut short (an
    /// image got no response and nothing is left) is the recorded fact behind
    /// `stopped: "network"`, as a refused fetch is. `data:` and refused
    /// (remote→local) URLs are not the wave's: they never reach the wire.
    ///
    /// [`image`]: Self::image
    pub fn fetch_images(&mut self, reqs: &[(String, bool)]) {
        let mut wave: Vec<(String, Intent)> = Vec::new();
        for (spec, background) in reqs {
            let Ok(url) = self.resolve(spec) else {
                continue;
            };
            let known = self.images.contains_key(&url) || wave.iter().any(|(u, _)| *u == url);
            if data::payload(&url).is_none() && !known {
                let background = *background;
                wave.push((url, Intent::Image { background }));
            }
        }
        let left = self.net.left().min(Duration::from_secs(TIMEOUT_SECS));
        let pool = self.budget.saturating_sub(self.spent);
        let (got, _) = self.net.charge(|| {
            fetch_many(
                &self.session,
                &wave,
                &self.base,
                Instant::now() + left,
                pool,
            )
        });
        self.refused |= got.len() < wave.len() && self.net.spent_out();
        for (i, r) in &got {
            let bytes = binary_bytes(&r.body);
            self.spent += bytes.len();
            let mime = header_value(&r.headers, "content-type").unwrap_or_default();
            self.images
                .insert(wave[*i].0.clone(), image_size(&mime, &bytes));
        }
        for (url, _) in wave {
            self.images.entry(url).or_insert(None);
        }
    }

    /// The size the image wave found for `url`; `None` when it failed, was
    /// never fetched, or is not an image frot can size.
    pub fn image(&self, url: &str) -> Option<(u32, u32)> {
        self.images.get(url).copied().flatten()
    }
}
