//! The one browser persona — single source of truth (`docs/design/identity.md`
//! §4). Every fact has one definition site here; tasks *derive*, storing nothing twice.
//!
//! This file holds the **shapes and the derivations**; the pinned values are the
//! §4.1 table and live alone in `pin.rs` (`bl-7523`), so §4.2's "a pin change is
//! an edit to §4.1 and nothing else" is one file's edit rather than a claim.
//!
//! `bl-abca` landed the **transport subset** (version, EOL tripwire I8, ALPN, TLS,
//! h2); the struct extends *additively* (`bl-20ec` headers, `bl-3972` navigator,
//! `bl-e707` clock, `bl-05e6` canvas seed, `bl-9663` chrome, `bl-ca20` screen). New fields.
//!
//! ## What Option C (Mark, 2026-07-21) makes this const mean
//!
//! frot's transport is stock `rustls 0.23` + `aws-lc-rs` — no ClientHello-crafting
//! fork (identity.md §6.1/§6.3). So these TLS/h2 targets are the **persona frot
//! aims at and the oracle measures against**, not a byte-exact Firefox handshake.
//! Where stock rustls emits its own shape, the gap is a *declared residual*
//! asserted in the §12 oracle, never a silent miss. `firefox_tls.rs` wires what
//! rustls can enforce and `transport.rs` what hyper can; the rest (cipher breadth,
//! extensions, key shares, `m,p,a,s`, two h2 SETTINGS) are §11/§6.4/§7/§12 residuals.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The Mozilla-compatibility `5.0` opening the `User-Agent` and `appVersion`: frozen
/// since 1998, a fact of the *derivations*, not the pin — one definition site (I1).
const MOZILLA_COMPAT: &str = "5.0";

/// The h2 wire facts of the persona (identity.md §3.1 akamai text, §12). Each
/// field's doc says whether frot **enforces** it on the wire, whether only the
/// `h2_preface.rs` oracle pins it, or whether it is a **declared residual** kept as
/// that oracle's reference — none claims an enforcement it lacks (`bl-f312`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H2Profile {
    /// SETTINGS_HEADER_TABLE_SIZE (id 1) — **declared residual**: absent from
    /// frot's wire, `hyper-util` exposes no setter (`transport.rs`).
    pub header_table_size: u32,
    /// SETTINGS_ENABLE_PUSH (id 2), off as in Firefox; oracle-pinned only (hyper's own value).
    pub enable_push: bool,
    /// SETTINGS_INITIAL_WINDOW_SIZE (id 4). Enforced.
    pub initial_window_size: u32,
    /// SETTINGS_MAX_FRAME_SIZE (id 5). Enforced.
    pub max_frame_size: u32,
    /// Connection WINDOW_UPDATE increment sent right after the preface. Enforced
    /// (`transport.rs` turns it into hyper's *target* window size).
    pub connection_window_increment: u32,
    /// Stream id the persona's first request HEADERS rides — **declared
    /// residual**: the `h2` crate opens client streams at 1 (`h2_request.rs`).
    pub initial_stream_id: u32,
    /// Persona pseudo-header order (oracle reference; stock `h2` sends `m,s,a,p`).
    pub pseudo_order: &'static str,
    /// Persona HEADERS PRIORITY weight (oracle reference; not emitted by `h2`).
    pub priority_weight: u8,
}

/// The TLS capability *targets* of the persona (identity.md §3.1/§3.2/§12), as
/// IANA numeric ids so this module stays free of `rustls` types. `firefox_tls.rs`
/// maps `groups`/`sig_algs` into the aws-lc-rs provider ordering (enforced) and
/// asserts the emitted ClientHello against the whole set (residuals flagged).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlsProfile {
    /// `supported_groups` target order: X25519MLKEM768, x25519, secp256r1,
    /// secp384r1, secp521r1, ffdhe2048, ffdhe3072. aws-lc-rs reaches the first
    /// four; secp521r1 and the two FFDHE groups are a declared residual.
    pub groups: &'static [u16],
    /// `key_share` target groups (4588, 29, 23). rustls emits ≤2 — the third is
    /// the §6.4 declared residual (JA4 does not encode key-share count).
    pub key_share_groups: &'static [u16],
    /// `signature_algorithms` target order (11 entries, identity.md §3.1). The
    /// two legacy SHA-1 schemes are a residual where the provider omits them.
    pub sig_algs: &'static [u16],
    /// The persona's advertised cipher suites in wire order (16, incl. legacy
    /// CBC/RSA). Stock rustls advertises only its AEAD suites, so the cipher list
    /// — and thus the JA4 cipher component — is a declared residual (Mark,
    /// 2026-07-21). Stored whole, not as a count, because `ja4.rs` hashes the
    /// list: a count beside it would be the same fact twice (I1/I6).
    pub ciphers: &'static [u16],
    /// The persona's ClientHello extension types in wire order (17, ECH last).
    /// **Declared residual in full** (§6.1): stock rustls emits its own set and
    /// order, so this is the oracle's reference for what frot does *not* send,
    /// and the JA4 extension-hash input.
    pub extensions: &'static [u16],
    /// `record_size_limit` (extension 28) value — **declared residual**: rustls
    /// sends no such extension (`recorder.rs` asserts its absence).
    pub record_size_limit: u16,
    /// `compress_certificate` (extension 27) algorithms — zlib(1), brotli(2),
    /// zstd(3) — **declared residual**: rustls sends no such extension.
    pub cert_compression: &'static [u16],
}

/// One browser persona. A pure `const`; construct nothing at runtime.
#[derive(Debug, Clone, Copy)]
pub struct BrowserProfile {
    /// Marketing name, e.g. `Firefox`.
    pub name: &'static str,
    /// Full pinned version string, e.g. `153.0esr`.
    pub version: &'static str,
    /// Major version — the number a UA string and `navigator` report.
    pub major: u16,
    /// Gecko `productSub` / build stamp (`20100101`).
    pub product_sub: &'static str,
    /// `navigator.platform` / oscpu token.
    pub platform: &'static str,
    /// The windowing-system token Gecko puts in the UA's first comment slot
    /// (`X11` on Linux) and **freezes `navigator.appVersion` at** — measured
    /// `5.0 (X11)` on 140esr and 153esr alike, so it tracks the platform, not
    /// the version (identity.md §8 ★). Stored, not derived: `Linux x86_64` does
    /// not yield `X11` without a per-OS map; it is read off the binary.
    pub window_system: &'static str,
    /// UI language (BCP-47), the sole `Accept-Language` / `navigator.language`.
    pub language: &'static str,
    /// JS `navigator.buildID` — Gecko's *privacy-frozen* constant, not the real
    /// BuildID (reporting which would itself be a tell). `bl-3972`, identity.md §8.
    pub build_id: &'static str,
    /// JS `navigator.hardwareConcurrency` — pinned low-entropy `8`, not the true
    /// host count (identity.md §8: entropy + determinism; `1` implausibly low).
    pub hardware_concurrency: u8,
    /// JS `screen`: `colorDepth`/`pixelDepth` (24-bit truecolor), and the
    /// `[width, height]` its `avail*` equal — no taskbar (identity.md §8).
    pub color_depth: u8,
    pub screen: [u16; 2],
    /// `outerHeight - innerHeight`, px: the toolbars above the viewport (`bl-9663`).
    pub chrome_height: u16,
    /// JS 2D-canvas fingerprint digest seed (`bl-05e6`, identity.md §11): a FIXED
    /// persona const (never host entropy) so `toDataURL`/`getImageData` are a
    /// deterministic function of (seed + draw sequence), stable across invocations.
    pub canvas_seed: u32,
    /// JS clock precision, µs — the `performance.now`/`Date.now` quantum (`bl-e707`).
    pub timer_precision_us: u32,
    /// End-of-life tripwire (I8): once `now` reaches this, the pin is stale and
    /// must be re-captured (§2). Stored as a civil date so the intent is legible.
    pub eol: CivilDate,
    /// ALPN offer, in wire order. Under Option C both entries are implemented, so
    /// the declared set equals the offered set (I2).
    pub alpn: &'static [&'static str],
    /// TLS capability targets.
    pub tls: TlsProfile,
    /// h2 wire facts.
    pub h2: H2Profile,
    /// `Accept` for a document navigation (identity.md §4.1): Firefox-shaped, no
    /// `image/avif,image/webp` (the Chrome tell §3.3 row 5 removes).
    pub accept_document: &'static str,
    /// `Accept` for an external stylesheet (`<link rel=stylesheet>`).
    pub accept_style: &'static str,
    /// `Accept` for scripts, modules, and `fetch`/XHR — Firefox sends `*/*`.
    pub accept_default: &'static str,
    /// `Accept` for an `<img>` (`bl-49f1`). It advertises AVIF, so frot sizes
    /// AVIF (I2: never advertise what you cannot speak).
    pub accept_image: &'static str,
}

/// A calendar date, compared without a date dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivilDate {
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

impl CivilDate {
    /// Days since the Unix epoch (1970-01-01 → 0), Howard Hinnant's algorithm
    /// specialised to the CE dates a persona pin uses (year ≥ 1), so the
    /// negative-year era adjustment of the general form is dropped rather than
    /// left as an unreachable branch. `const` so the tripwire is compile-time.
    /// `pub(crate)` so the cookie jar's `Expires` parser (`super::cookie`) reuses
    /// the one civil→epoch algorithm rather than growing a second copy.
    pub(crate) const fn days_since_epoch(self) -> i64 {
        let (y, m, d) = (self.year, self.month, self.day);
        let y = if m <= 2 { y - 1 } else { y };
        let era = y / 400;
        let yoe = y - era * 400;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }
}

impl BrowserProfile {
    /// Whether the pin is still current at `now` (I8). `now` is injected so the
    /// tripwire is testable on both sides without waiting for the wall clock.
    pub fn is_current(&self, now: SystemTime) -> bool {
        let today = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as i64
            / 86_400;
        today < self.eol.days_since_epoch()
    }

    /// The HTTP `User-Agent` this persona sends, and the sole source
    /// `navigator.userAgent` derives from (identity.md §4.2). Built from the
    /// pinned `major`, `window_system`, `platform` and `product_sub` so a re-pin
    /// (§4.1) has one edit and nothing to regenerate — an ESR UA reports
    /// `rv:<major>.0` / `Firefox/<major>.0`, and its `Gecko/` trail *is*
    /// `productSub`, which is why the two cannot drift apart (I1).
    pub fn user_agent(&self) -> String {
        format!(
            "Mozilla/{MOZILLA_COMPAT} ({}; {}; rv:{}.0) Gecko/{} {}/{}.0",
            self.window_system, self.platform, self.major, self.product_sub, self.name, self.major
        )
    }

    /// `navigator.appVersion` (identity.md §4.2). Gecko freezes it at the
    /// Mozilla-compatibility version plus the *windowing-system* token — `5.0
    /// (X11)` — **not** the UA minus its `Mozilla/` prefix, which is what this
    /// used to compute and what §8's measured table always denied. It derives
    /// from the pin, not the effective UA, per I5: a `-H "User-Agent: …"`
    /// override replaces the UA and "changes nothing else".
    pub fn app_version(&self) -> String {
        format!("{MOZILLA_COMPAT} ({})", self.window_system)
    }

    /// The `Accept-Language` header, derived from `language` (identity.md §4.2:
    /// `en-US,en;q=0.9` is Gecko's rendering of the `en-US` UI locale — measured
    /// 2026-08-11 on the 153esr pin, which raised the weight 140esr sent at
    /// `q=0.5`). The base language rides at `q=0.9`; a bare locale with no region
    /// degrades to itself.
    pub fn accept_language(&self) -> String {
        match self.language.split_once('-') {
            Some((base, _)) => format!("{},{base};q=0.9", self.language),
            None => self.language.to_string(),
        }
    }
}

/// The one pinned persona (§4.1) — its own file, so a re-pin is one file's edit.
mod pin;

/// The per-[`Intent`](super::Intent) request metadata derived from the persona.
mod meta;

pub use pin::FIREFOX_153_ESR;

#[cfg(test)]
mod tests;
