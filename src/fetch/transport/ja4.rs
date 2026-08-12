//! JA4 — computed, never written down (identity.md §12 "Hashes are assertions,
//! not fixtures"; §4.2 "nothing to regenerate").
//!
//! This is the *function*, not a fixture: it maps an ordered set of ClientHello
//! capabilities to the FoxIO JA4 string and to its unhashed `JA4_r`/`JA4_ro`
//! forms. `recorder.rs` runs it twice per test — once over the ClientHello frot
//! actually emitted, once over the persona's §4.1 lists — so neither side is a
//! literal and a re-pin moves both. That is what makes §4.2's "a pin change is a
//! one-table edit … there is nothing to regenerate" a tested claim rather than
//! an aspiration: no JA4 string exists anywhere in the tree to regenerate.
//!
//! Scope, honestly: JA3/JA3N are MD5-based, and peetprint is one service's
//! undocumented format — see the §12 correction of 2026-08-12 for why neither is
//! computed here. The h2 half of the same claim is `h2_wire::akamai`.

use rustls::crypto::aws_lc_rs::cipher_suite::TLS13_AES_128_GCM_SHA256;

/// The ordered ClientHello facts JA4 reads. Borrowed, so the same struct
/// describes a parsed capture and the profile's declared lists.
pub(super) struct Ja4Hello<'a> {
    /// Highest offered TLS version (`supported_versions`, GREASE removed).
    pub version: u16,
    /// Whether a `server_name` extension is present (`d` vs `i`).
    pub sni: bool,
    /// Cipher suites, wire order.
    pub ciphers: &'a [u16],
    /// Extension types, wire order.
    pub extensions: &'a [u16],
    /// `signature_algorithms`, wire order — never sorted, even in JA4_r.
    pub sig_algs: &'a [u16],
    /// First ALPN protocol offered.
    pub alpn: &'a str,
}

/// Whether `v` is a GREASE value — RFC 8701's `0x0a0a, 0x1a1a, … 0xfafa`: both
/// nibble pairs are `a`, and the two bytes are equal. JA4 ignores them
/// everywhere, in the count as well as the hash. `recorder.rs` asserts the
/// *absence* of GREASE through this same predicate, so the class is defined once.
pub(super) fn is_grease(v: u16) -> bool {
    v & 0x0f0f == 0x0a0a && v >> 12 == (v >> 4) & 0xf
}

/// JA4's two-digit TLS version code. Only 1.3 and 1.2 are named: the persona and
/// frot both offer 1.3, and 1.2 is kept so a downgrade reads as a *different*
/// fingerprint instead of collapsing into the `00` unknown.
fn version_code(version: u16) -> &'static str {
    match version {
        0x0304 => "13",
        0x0303 => "12",
        _ => "00",
    }
}

/// `4a3f,0035,…` — a comma-joined lowercase-hex list, GREASE dropped, sorted
/// for JA4/JA4_r and left in wire order for JA4_ro.
fn field(values: &[u16], sorted: bool) -> String {
    let mut kept: Vec<u16> = values.iter().copied().filter(|&v| !is_grease(v)).collect();
    if sorted {
        kept.sort_unstable();
    }
    kept.iter()
        .map(|v| format!("{v:04x}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The first 12 hex characters of SHA-256, JA4's truncation. The hash comes from
/// the TLS 1.3 suite rustls already links, so computing a fingerprint adds no
/// dependency (`Cargo.toml` escalation rule, AGENTS.md).
fn sha12(input: &str) -> String {
    let suite = TLS13_AES_128_GCM_SHA256
        .tls13()
        .expect("TLS13_AES_128_GCM_SHA256 is a TLS 1.3 suite");
    suite.common.hash_provider.hash(input.as_bytes()).as_ref()[..6]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl Ja4Hello<'_> {
    /// `JA4_a` — `t13d1617h2`: TCP, version, SNI presence, cipher count,
    /// extension count (SNI and ALPN *are* counted here), first ALPN's first and
    /// last character.
    ///
    /// The two counts are 2-digit by the spec; the persona's inputs are 16 and
    /// 17 entries, far under the spec's 99 clamp, so no clamp is implemented — a
    /// persona with a 100-entry list would need one.
    fn header(&self) -> String {
        let ciphers = self.ciphers.iter().filter(|&&v| !is_grease(v)).count();
        let extensions = self.extensions.iter().filter(|&&v| !is_grease(v)).count();
        let sni = if self.sni { 'd' } else { 'i' };
        let first = self.alpn.chars().next().unwrap_or('0');
        let last = self.alpn.chars().last().unwrap_or('0');
        format!(
            "t{}{sni}{ciphers:02}{extensions:02}{first}{last}",
            version_code(self.version)
        )
    }

    /// The extension half's input: extensions minus `server_name` (0) and ALPN
    /// (16) — dropped so one client fingerprints the same on every domain — then
    /// `_` and the signature algorithms, which keep wire order in every form.
    fn extension_field(&self, sorted: bool) -> String {
        let kept: Vec<u16> = self
            .extensions
            .iter()
            .copied()
            .filter(|&v| v != 0x0000 && v != 0x0010)
            .collect();
        format!("{}_{}", field(&kept, sorted), field(self.sig_algs, false))
    }

    /// `JA4_r` (sorted) or `JA4_ro` (wire order) — the unhashed forms, which are
    /// what a diff between two of these can actually be read from.
    pub(super) fn raw(&self, sorted: bool) -> String {
        format!(
            "{}_{}_{}",
            self.header(),
            field(self.ciphers, sorted),
            self.extension_field(sorted)
        )
    }

    /// `JA4` proper — the header with both halves hashed.
    pub(super) fn ja4(&self) -> String {
        format!(
            "{}_{}_{}",
            self.header(),
            sha12(&field(self.ciphers, true)),
            sha12(&self.extension_field(true))
        )
    }
}

#[test]
fn grease_is_dropped_from_every_component_and_non_grease_lookalikes_are_kept() {
    // 0x1a1a is GREASE; 0x2a1a shares the low nibbles but not the byte pair, and
    // 0x1301 shares neither — both must survive. Covers both halves of the test.
    let hello = Ja4Hello {
        version: 0x0304,
        sni: true,
        ciphers: &[0x1a1a, 0x1301, 0x2a1a],
        extensions: &[0x0a0a, 0x0000, 0x0010, 0x002b],
        sig_algs: &[0x0403],
        alpn: "h2",
    };
    // Counts exclude GREASE (2 ciphers, 3 extensions) but not SNI/ALPN — which
    // are counted in JA4_a and then dropped from the hashed extension list.
    assert_eq!(hello.raw(true), "t13d0203h2_1301,2a1a_002b_0403");
}

#[test]
fn the_unsorted_form_keeps_wire_order_and_an_unknown_version_reads_as_zero() {
    let hello = Ja4Hello {
        version: 0x0301,
        sni: false,
        ciphers: &[0x1302, 0x1301],
        extensions: &[0x002b, 0x000d],
        sig_algs: &[0x0403, 0x0201],
        alpn: "",
    };
    assert_eq!(hello.raw(false), "t00i020200_1302,1301_002b,000d_0403,0201");
    // Sorting is the only difference between JA4_r and JA4_ro; an absent ALPN
    // degrades to `00` rather than panicking on an empty string.
    assert_eq!(hello.raw(true), "t00i020200_1301,1302_000d,002b_0403,0201");
    // TLS 1.2 is named rather than unknown, so a downgrade is a distinct string.
    assert_eq!(version_code(0x0303), "12");
}

#[test]
fn ja4_hashes_each_half_to_twelve_hex_characters_of_sha256() {
    let hello = Ja4Hello {
        version: 0x0304,
        sni: true,
        ciphers: &[0x1301],
        extensions: &[0x002b],
        sig_algs: &[0x0403],
        alpn: "h2",
    };
    let ja4 = hello.ja4();
    let parts: Vec<&str> = ja4.split('_').collect();
    assert_eq!(parts[0], "t13d0101h2");
    // `sha256("1301")` = 0f2cb44170f4…, truncated to 12 hex characters. This is
    // the one fixture in the module and it is deliberately *not* a persona fact:
    // it pins the hash **function** (SHA-256, first 6 bytes, lowercase hex), so
    // a provider swap that changed the digest fails here rather than silently
    // changing every fingerprint the oracle computes.
    assert_eq!(parts[1], "0f2cb44170f4");
    assert_eq!(parts[2].len(), 12, "{ja4}");
}
