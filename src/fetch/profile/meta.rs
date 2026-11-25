//! The per-request-class header facts, derived from one [`Intent`]
//! (identity.md §4.1/§4.2) — split from `profile.rs` so the shapes and the
//! derivation of the one persona each stay one file.

use super::super::Intent;
use super::BrowserProfile;

/// The per-request-class header facts the persona sends (identity.md §4.1/§4.2),
/// derived from one [`Intent`]. `Accept`, `Sec-Fetch-Dest`/`-Mode`, whether a
/// `Sec-Fetch-User`/`Upgrade-Insecure-Requests` rides, and the RFC 9218
/// `Priority` all follow from the request's role; `Sec-Fetch-Site` and `Referer`
/// depend on URL facts and are computed in `request.rs`, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestMeta {
    /// `Accept` value for this class.
    pub accept: &'static str,
    /// `Sec-Fetch-Dest` (Fetch Metadata destination).
    pub dest: &'static str,
    /// `Sec-Fetch-Mode` (Fetch Metadata mode).
    pub mode: &'static str,
    /// `Sec-Fetch-User` — `Some("?1")` only for a user-activated navigation.
    pub user: Option<&'static str>,
    /// Whether `Upgrade-Insecure-Requests: 1` rides (navigations only).
    pub uir: bool,
    /// RFC 9218 `Priority`: document `u=0, i` (§3.3 row 8); render-blocking
    /// leaders (CSS, blocking scripts, modules) `u=2`; `fetch`/XHR `u=4`; an
    /// `<img>` `u=4, i`, or `u=5, i` detached or lazy (identity.md §3.20).
    pub priority: &'static str,
}

impl BrowserProfile {
    /// The per-class request metadata for one [`Intent`] (identity.md §4.1). The
    /// one place navigation vs style vs classic-script vs module vs fetch/XHR
    /// diverge in `Accept`, Fetch Metadata, and `Priority`.
    pub fn request_meta(&self, intent: Intent) -> RequestMeta {
        match intent {
            Intent::Navigation => RequestMeta {
                accept: self.accept_document,
                dest: "document",
                mode: "navigate",
                user: Some("?1"),
                uir: true,
                priority: "u=0, i",
            },
            Intent::Style => RequestMeta {
                accept: self.accept_style,
                dest: "style",
                mode: "no-cors",
                user: None,
                uir: false,
                priority: "u=2",
            },
            Intent::ClassicScript => RequestMeta {
                accept: self.accept_default,
                dest: "script",
                mode: "no-cors",
                user: None,
                uir: false,
                priority: "u=2",
            },
            Intent::Module => RequestMeta {
                accept: self.accept_default,
                dest: "script",
                mode: "cors",
                user: None,
                uir: false,
                priority: "u=2",
            },
            Intent::FetchXhr => RequestMeta {
                accept: self.accept_default,
                dest: "empty",
                mode: "cors",
                user: None,
                uir: false,
                priority: "u=4",
            },
            Intent::Image { background } => RequestMeta {
                accept: self.accept_image,
                dest: "image",
                mode: "no-cors",
                user: None,
                uir: false,
                priority: if background { "u=5, i" } else { "u=4, i" },
            },
        }
    }
}
