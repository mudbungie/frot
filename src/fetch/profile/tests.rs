//! The persona const is the single source of truth; these lock the I8 tripwire
//! (`is_current`) and the date arithmetic behind it, both sides of every branch.

use super::*;

/// A `SystemTime` at midnight UTC of `date`, built through the same civil-date
/// arithmetic the tripwire uses, so the test states a calendar day, not a magic
/// second count.
fn at(date: CivilDate) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs((date.days_since_epoch() * 86_400) as u64)
}

#[test]
fn pin_is_current_the_day_before_eol() {
    // Derived from the const rather than spelled out: the eol date keeps one
    // definition site (I1), so a re-pin (§4.1) cannot leave a stale literal here.
    let day_before = at(FIREFOX_153_ESR.eol) - Duration::from_secs(86_400);
    assert!(FIREFOX_153_ESR.is_current(day_before));
}

#[test]
fn pin_expires_on_its_eol_day() {
    // On the eol day itself `today < eol` is false — the tripwire fires.
    assert!(!FIREFOX_153_ESR.is_current(at(FIREFOX_153_ESR.eol)));
}

#[test]
fn a_pre_epoch_clock_is_treated_as_day_zero() {
    // `duration_since(UNIX_EPOCH)` errors before 1970; the fallback pins today at
    // day 0, which is still current. Covers the error arm of `is_current`.
    let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    assert!(FIREFOX_153_ESR.is_current(before_epoch));
}

#[test]
fn days_since_epoch_anchors_and_spans_both_month_branches() {
    // Month <= 2 branch (January) and the epoch anchor in one assertion.
    assert_eq!(
        CivilDate {
            year: 1970,
            month: 1,
            day: 1,
        }
        .days_since_epoch(),
        0
    );
    // Month > 2 branch, a full leap-cycle out, cross-checked against a known
    // ordinal (2000-03-01 is 11017 days after the epoch).
    assert_eq!(
        CivilDate {
            year: 2000,
            month: 3,
            day: 1,
        }
        .days_since_epoch(),
        11017
    );
}

#[test]
fn transport_targets_are_the_pinned_persona_facts() {
    // Guards the const against silent drift of the facts firefox_tls enforces.
    assert_eq!(FIREFOX_153_ESR.alpn, &["h2", "http/1.1"]);
    assert_eq!(FIREFOX_153_ESR.tls.groups[0], 4588);
    assert_eq!(FIREFOX_153_ESR.tls.key_share_groups, &[4588, 29, 23]);
    assert_eq!(FIREFOX_153_ESR.tls.sig_algs.len(), 11);
    // The §4.1 ordered lists, whose *contents* the `recorder.rs`/`ja4.rs` oracle
    // reads. §12 pins the one ClientHello fact that moved 140esr → 153esr — the
    // **absence** of `0xc009`, which 140esr carried at index 10 — and ECH last in
    // the extension list; both are checked here rather than restated as a second
    // copy of the whole table.
    assert_eq!(FIREFOX_153_ESR.tls.ciphers.len(), 16);
    assert!(!FIREFOX_153_ESR.tls.ciphers.contains(&0xc009));
    assert_eq!(FIREFOX_153_ESR.tls.extensions.len(), 17);
    assert_eq!(FIREFOX_153_ESR.tls.extensions[16], 65037);
    assert_eq!(FIREFOX_153_ESR.tls.record_size_limit, 16_385);
    assert_eq!(FIREFOX_153_ESR.tls.cert_compression, &[1, 2, 3]);
    assert_eq!(
        FIREFOX_153_ESR.h2,
        H2Profile {
            header_table_size: 65_536,
            enable_push: false,
            initial_window_size: 131_072,
            max_frame_size: 16_384,
            connection_window_increment: 12_517_377,
            initial_stream_id: 3,
            pseudo_order: "m,p,a,s",
            priority_weight: 42,
        }
    );
    assert_eq!(FIREFOX_153_ESR.major, 153);
    assert_eq!(FIREFOX_153_ESR.product_sub, "20100101");
    assert_eq!(FIREFOX_153_ESR.platform, "Linux x86_64");
    assert_eq!(FIREFOX_153_ESR.window_system, "X11");
    assert_eq!(FIREFOX_153_ESR.language, "en-US");
    assert_eq!(FIREFOX_153_ESR.name, "Firefox");
    assert_eq!(FIREFOX_153_ESR.version, "153.0esr");
    // The canvas digest seed is a fixed persona constant (bl-05e6): pinned here so
    // a silent change — which would shift every deterministic canvas hash — fails.
    assert_eq!(FIREFOX_153_ESR.canvas_seed, 0x_f00d_c0de);
}

#[test]
fn derived_headers_come_from_the_pinned_facts() {
    // UA, appVersion and Accept-Language are computed from
    // `window_system`/`platform`/`major`/`product_sub`/`language`, so a re-pin
    // needs no second edit (§4.2). The region `en-US` rides at q=0.9.
    assert_eq!(
        FIREFOX_153_ESR.user_agent(),
        "Mozilla/5.0 (X11; Linux x86_64; rv:153.0) Gecko/20100101 Firefox/153.0"
    );
    // Gecko freezes appVersion at the Mozilla-compat version + the WINDOWING
    // token — measured `5.0 (X11)` on 140esr and 153esr alike (identity.md §8 ★).
    // It does not track the version and is not the UA minus `Mozilla/` (bl-6491).
    assert_eq!(FIREFOX_153_ESR.app_version(), "5.0 (X11)");
    assert_ne!(
        FIREFOX_153_ESR.app_version(),
        FIREFOX_153_ESR.user_agent()["Mozilla/".len()..]
    );
    // A re-pin moves the version and the UA; appVersion must not follow it.
    let mut next = FIREFOX_153_ESR;
    next.major = 160;
    assert!(next.user_agent().contains("rv:160.0"));
    assert_eq!(next.app_version(), "5.0 (X11)");
    // A different windowing system moves both together, which is the point of the
    // one stored token: `X11` never appears twice in the tree (I1).
    next.window_system = "Windows";
    assert_eq!(next.app_version(), "5.0 (Windows)");
    assert!(next.user_agent().starts_with("Mozilla/5.0 (Windows; "));
    assert_eq!(FIREFOX_153_ESR.accept_language(), "en-US,en;q=0.9");
    // A region-less locale degrades to itself (the fallback arm).
    let mut bare = FIREFOX_153_ESR;
    bare.language = "en";
    assert_eq!(bare.accept_language(), "en");
    // Every intent maps to its own destination/mode; navigation alone is a
    // user-activated, upgrade-insecure document.
    assert_eq!(
        FIREFOX_153_ESR.request_meta(Intent::Navigation).dest,
        "document"
    );
    assert!(FIREFOX_153_ESR.request_meta(Intent::Navigation).uir);
    assert_eq!(FIREFOX_153_ESR.request_meta(Intent::FetchXhr).user, None);
}
