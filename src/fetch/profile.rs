//! The one browser persona — single source of truth (`docs/design/identity.md`
//! §4). Every fact has one definition site here; tasks *derive*, storing nothing twice.
//!
//! This ball (`bl-abca`) landed the **transport subset** (version, EOL tripwire
//! I8, ALPN order, TLS targets, h2 facts). The struct extends *additively* —
//! `bl-20ec` request headers, `bl-3972` the JS `navigator` facts, `bl-e707` clock
//! precision, `bl-05e6` the canvas seed. New fields, not new files.
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

use super::Intent;

/// The h2 wire facts of the persona (identity.md §3.1 akamai text, §12). Each
/// field's doc says whether frot **enforces** it on the wire, whether only the
/// `h2_preface.rs` oracle pins it, or whether it is a **declared residual** kept as
/// that oracle's reference — none claims an enforcement it lacks (`bl-f312`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H2Profile {
    /// SETTINGS_HEADER_TABLE_SIZE (id 1) — **declared residual**: absent from
    /// frot's wire, `hyper-util` exposes no setter (`transport.rs`).
    pub header_table_size: u32,
    /// SETTINGS_ENABLE_PUSH (id 2) — Firefox disables push. Oracle-pinned only;
    /// hyper hardcodes the same value.
    pub enable_push: bool,
    /// SETTINGS_INITIAL_WINDOW_SIZE (id 4). Enforced.
    pub initial_window_size: u32,
    /// SETTINGS_MAX_FRAME_SIZE (id 5). Enforced.
    pub max_frame_size: u32,
    /// Connection WINDOW_UPDATE increment sent right after the preface. Enforced
    /// (`transport.rs` turns it into hyper's *target* window size).
    pub connection_window_increment: u32,
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
    /// The persona's advertised cipher count (17, incl. legacy CBC/RSA). Stock
    /// rustls advertises only its AEAD suites, so the cipher list — and thus the
    /// JA4 cipher component — is a declared residual (Mark, 2026-07-21).
    pub cipher_count: u8,
}

/// One browser persona. A pure `const`; construct nothing at runtime.
#[derive(Debug, Clone, Copy)]
pub struct BrowserProfile {
    /// Marketing name, e.g. `Firefox`.
    pub name: &'static str,
    /// Full pinned version string, e.g. `140.12.0esr`.
    pub version: &'static str,
    /// Major version — the number a UA string and `navigator` report.
    pub major: u16,
    /// Gecko `productSub` / build stamp (`20100101`).
    pub product_sub: &'static str,
    /// `navigator.platform` / oscpu token.
    pub platform: &'static str,
    /// UI language (BCP-47), the sole `Accept-Language` / `navigator.language`.
    pub language: &'static str,
    /// JS `navigator.buildID` — Gecko's *privacy-frozen* constant, not the real
    /// BuildID (reporting which would itself be a tell). `bl-3972`, identity.md §8.
    pub build_id: &'static str,
    /// JS `navigator.hardwareConcurrency` — pinned low-entropy `8`, not the true
    /// host count (identity.md §8: entropy + determinism; `1` implausibly low).
    pub hardware_concurrency: u8,
    /// JS `screen.colorDepth`/`pixelDepth` — 24-bit truecolor (identity.md §8).
    pub color_depth: u8,
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
    /// `Accept` for a document navigation (identity.md §4.1 `accept_document`).
    /// Firefox-shaped: no `image/avif,image/webp` (that pair is the Chrome tell
    /// §3.3 row 5 removes).
    pub accept_document: &'static str,
    /// `Accept` for an external stylesheet (`<link rel=stylesheet>`).
    pub accept_style: &'static str,
    /// `Accept` for scripts, modules, and `fetch`/XHR — Firefox sends `*/*`.
    pub accept_default: &'static str,
}

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
    /// leaders (CSS, blocking scripts, modules) `u=2`; `fetch`/XHR `u=4`.
    pub priority: &'static str,
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
    /// pinned `major` and `platform` so a re-pin (§4.1) has exactly one edit and
    /// nothing to regenerate — an ESR UA reports `rv:<major>.0` / `Firefox/<major>.0`.
    pub fn user_agent(&self) -> String {
        format!(
            "Mozilla/5.0 (X11; {}; rv:{}.0) Gecko/20100101 Firefox/{}.0",
            self.platform, self.major, self.major
        )
    }

    /// The `Accept-Language` header, derived from `language` (identity.md §4.2:
    /// `en-US,en;q=0.5` is Gecko's rendering of the `en-US` UI locale). The base
    /// language rides at `q=0.5`; a bare locale with no region degrades to itself.
    pub fn accept_language(&self) -> String {
        match self.language.split_once('-') {
            Some((base, _)) => format!("{},{base};q=0.5", self.language),
            None => self.language.to_string(),
        }
    }

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
        }
    }
}

/// The pinned persona: Mozilla Firefox 140.12.0esr, linux-x86_64 en-US
/// (identity.md §2). Captured 2026-07-19 via Marionette from Gecko's own
/// necko/NSS stack.
pub const FIREFOX_140_ESR: BrowserProfile = BrowserProfile {
    name: "Firefox",
    version: "140.12.0esr",
    major: 140,
    product_sub: "20100101",
    platform: "Linux x86_64",
    language: "en-US",
    build_id: "20181001000000",
    hardware_concurrency: 8,
    color_depth: 24,
    canvas_seed: 0x_f00d_c0de, // opaque fixed seed; mixed with draws, never raw.
    timer_precision_us: 1_000, // 1 ms — Firefox's reduceTimerPrecision default
    // ESR 140 is the sole current ESR line (`FIREFOX_ESR_NEXT` empty, §2); when
    // its successor lands this trips and forces a re-capture + re-pin.
    eol: CivilDate {
        year: 2027,
        month: 6,
        day: 1,
    },
    alpn: &["h2", "http/1.1"],
    tls: TlsProfile {
        // 4588=X25519MLKEM768, 29=x25519, 23=secp256r1, 24=secp384r1,
        // 25=secp521r1, 256=ffdhe2048, 257=ffdhe3072.
        groups: &[4588, 29, 23, 24, 25, 256, 257],
        key_share_groups: &[4588, 29, 23],
        // ecdsa_secp256r1_sha256, ecdsa_secp384r1_sha384, ecdsa_secp521r1_sha512,
        // rsa_pss_rsae_sha256/384/512, rsa_pkcs1_sha256/384/512,
        // ecdsa_sha1, rsa_pkcs1_sha1.
        sig_algs: &[
            0x0403, 0x0503, 0x0603, 0x0804, 0x0805, 0x0806, 0x0401, 0x0501, 0x0601, 0x0203, 0x0201,
        ],
        cipher_count: 17,
    },
    h2: H2Profile {
        header_table_size: 65_536,
        enable_push: false,
        initial_window_size: 131_072,
        max_frame_size: 16_384,
        connection_window_increment: 12_517_377,
        pseudo_order: "m,p,a,s",
        priority_weight: 42,
    },
    accept_document: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    accept_style: "text/css,*/*;q=0.1",
    accept_default: "*/*",
};

#[cfg(test)]
mod tests;
