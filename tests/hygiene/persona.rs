//! The pinned browser persona has not gone stale (`bl-04ab`, identity.md I8).
//!
//! `BrowserProfile::is_current` calls itself "the I8 tripwire", and I8 says a
//! test asserts `today < profile.eol`. It did not: the function had no
//! production caller and every test caller fabricated a `SystemTime`, so
//! nothing ever asked whether the pin is stale **today** and the pin could slide
//! past its end-of-life with no gate noticing. This is the missing caller — the
//! one that reads the wall clock.
//!
//! It lives here, not beside the const, because it is an invariant of the
//! *checkout* (like [`super::size`]'s line cap): it fails on the calendar, not
//! on an edit, and the cheapest place to read that failure is `cargo test`. The
//! fabricated clocks in `src/fetch/profile/tests.rs` stay — they pin the date
//! *arithmetic* on both sides of the boundary, which is exactly what a
//! wall-clock test cannot do, and equally what they cannot do is notice a date
//! that has simply arrived.

use frot::fetch::FIREFOX_140_ESR;
use std::time::{Duration, SystemTime};

/// How far out an end-of-life can honestly sit. Mozilla ships roughly one ESR
/// line a year (identity.md §2), so an `eol` two years out is not a date off
/// the ESR calendar — it is the tripwire snoozed.
const HORIZON: Duration = Duration::from_secs(2 * 365 * 24 * 60 * 60);

/// What to do when either gate below fires. Both failures have the same remedy,
/// and the remedy is one edit, so it is written once.
const REPIN: &str = "Re-capture a current ESR line and re-pin it in \
    src/fetch/profile.rs per docs/design/identity.md §4.1: the profile const is \
    the single definition site, so the re-pin is that one edit with nothing \
    derived to regenerate (§4.2). Read §3.8 before you do — the JA4 cipher hash \
    and the ClientHello extension set move with the line.";

#[test]
fn the_pinned_persona_is_current_today() {
    assert!(
        FIREFOX_140_ESR.is_current(SystemTime::now()),
        "the pinned persona ({} {}) is past the end-of-life recorded beside it. \
         {REPIN}",
        FIREFOX_140_ESR.name,
        FIREFOX_140_ESR.version
    );
}

#[test]
fn the_tripwire_still_fires_within_one_esr_horizon() {
    // Negative control: without it the gate above passes whether or not it can
    // ever fail — a tripwire wired to `true`, or an `eol` pushed out to a date
    // no ESR line reaches, both read as "the pin is current". Rolling the *real*
    // clock forward by the horizon closes both holes at once: the pin must go
    // stale within a span the ESR calendar actually spans.
    assert!(
        !FIREFOX_140_ESR.is_current(SystemTime::now() + HORIZON),
        "the pin still reads current two years out, so this gate can no longer \
         fail: either `is_current` stopped discriminating, or `eol` was pushed \
         past the ESR calendar (roughly one line a year, identity.md §2) rather \
         than the pin being refreshed. {REPIN}"
    );
}
