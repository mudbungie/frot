//! The one browser persona — single source of truth (`docs/design/identity.md`
//! §4). Every persona fact has exactly one definition site here; downstream
//! tasks *derive* from this const and store nothing twice.
//!
//! This ball (`bl-abca`) lands the **transport subset**: the pinned version, the
//! end-of-life tripwire (I8), the ALPN target order, the TLS capability targets,
//! and the h2 wire facts. The struct is designed to be extended *additively* —
//! `bl-20ec` adds the ordered request-header description (and rewires the live
//! `User-Agent`, which this ball deliberately does **not** touch), `bl-3972` the
//! JS-facing `navigator` facts, `bl-e707` the clock precision. New fields, not
//! new files.
//!
//! ## What Option C (Mark, 2026-07-21) makes this const mean
//!
//! frot's transport is stock `rustls 0.23` + `aws-lc-rs` — no ClientHello-crafting
//! fork (identity.md §6.1/§6.3). So these TLS/h2 targets are the **persona frot
//! aims at and the oracle measures against**, not a promise of a byte-exact
//! Firefox handshake. Where stock rustls emits its own shape, the gap is a
//! *declared residual* asserted in the §12 oracle, never a silent miss. The facts
//! frot genuinely enforces from here — ALPN order, kx-group order, signature-
//! algorithm order, the h2 SETTINGS values — are wired through `firefox_tls.rs`;
//! the rest (cipher list breadth, extension order/set, three key shares,
//! `m,p,a,s` pseudo-order) are the residuals §11/§6.4/§7 enumerate.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The h2 wire facts of the persona (identity.md §3.1 akamai text, §12). The
/// SETTINGS values and the connection-level WINDOW_UPDATE increment are
/// *enforced* on the h2 client; `pseudo_order` and the HEADERS `PRIORITY` weight
/// are the persona's values kept here as the **oracle's expected-residual
/// reference** — the stock `h2` crate emits `m,s,a,p` and no client PRIORITY, so
/// those two are declared residuals (§7 stage C), not what frot sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H2Profile {
    /// SETTINGS_HEADER_TABLE_SIZE (id 1).
    pub header_table_size: u32,
    /// SETTINGS_ENABLE_PUSH (id 2) — Firefox disables push.
    pub enable_push: bool,
    /// SETTINGS_INITIAL_WINDOW_SIZE (id 4).
    pub initial_window_size: u32,
    /// SETTINGS_MAX_FRAME_SIZE (id 5).
    pub max_frame_size: u32,
    /// The connection-level WINDOW_UPDATE increment sent right after the preface.
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
    const fn days_since_epoch(self) -> i64 {
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
};

#[cfg(test)]
mod tests;
