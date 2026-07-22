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
    let now = at(CivilDate {
        year: 2027,
        month: 5,
        day: 31,
    });
    assert!(FIREFOX_140_ESR.is_current(now));
}

#[test]
fn pin_expires_on_its_eol_day() {
    // On the eol day itself `today < eol` is false — the tripwire fires.
    assert!(!FIREFOX_140_ESR.is_current(at(FIREFOX_140_ESR.eol)));
}

#[test]
fn a_pre_epoch_clock_is_treated_as_day_zero() {
    // `duration_since(UNIX_EPOCH)` errors before 1970; the fallback pins today at
    // day 0, which is still current. Covers the error arm of `is_current`.
    let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    assert!(FIREFOX_140_ESR.is_current(before_epoch));
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
    assert_eq!(FIREFOX_140_ESR.alpn, &["h2", "http/1.1"]);
    assert_eq!(FIREFOX_140_ESR.tls.groups[0], 4588);
    assert_eq!(FIREFOX_140_ESR.tls.key_share_groups, &[4588, 29, 23]);
    assert_eq!(FIREFOX_140_ESR.tls.sig_algs.len(), 11);
    assert_eq!(FIREFOX_140_ESR.tls.cipher_count, 17);
    assert_eq!(
        FIREFOX_140_ESR.h2,
        H2Profile {
            header_table_size: 65_536,
            enable_push: false,
            initial_window_size: 131_072,
            max_frame_size: 16_384,
            connection_window_increment: 12_517_377,
            pseudo_order: "m,p,a,s",
            priority_weight: 42,
        }
    );
    assert_eq!(FIREFOX_140_ESR.major, 140);
    assert_eq!(FIREFOX_140_ESR.product_sub, "20100101");
    assert_eq!(FIREFOX_140_ESR.platform, "Linux x86_64");
    assert_eq!(FIREFOX_140_ESR.language, "en-US");
    assert_eq!(FIREFOX_140_ESR.name, "Firefox");
    assert_eq!(FIREFOX_140_ESR.version, "140.12.0esr");
}
