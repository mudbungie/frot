//! §5/§6 network-bound proofs (`bl-79dc`): `NET_BUDGET_MS` is **wire** time, not
//! a wall window over the whole run, so §10 `stopped: "network"` names a network
//! that was actually slow.
//!
//! The two cases the fix is measured against are here: a run that spends its
//! wall time on **compute** and then subfetches (which must *not* blame the
//! network), and a run that spends it genuinely **waiting on a slow origin**
//! (which must). Both are exact: the "slow origin" is a `mockito` handler that
//! advances the run's own [`Clock::manual`], and the "compute" is an advance of
//! that same clock between arming and running — what `Clock::advance` is for.
//! Nothing sleeps and nothing spins, so neither verdict can be changed by host
//! load, which is the defect's own moral.

use std::time::Duration;

use mockito::{Mock, Server};

use super::super::engine::{Clock, Deadline, NetBudget, EXEC_CPU_MS};
use super::super::{run_session, Env, Report, Session, StyleSource};
use crate::dom::Document;
use crate::envelope::JsStop;
use crate::fetch::FetchSession;

/// The §6 budget these proofs run under: a tenth of a second of wire time,
/// metered on a clock only the test and its mock server move.
fn budget(clock: &Clock) -> NetBudget {
    NetBudget::on(clock.clone(), Duration::from_millis(100))
}

/// Run `html` at `url` under `net`, doing `between` after the run is armed and
/// before the page executes — the seam a stipulated stretch of *compute* is
/// injected through. Compute rides a frozen clock, so only the network bound is
/// ever in question here.
fn run_at(html: &str, url: &str, net: NetBudget, between: impl FnOnce()) -> Report {
    let fetch = FetchSession::new(Vec::new());
    let session = Session::with_bounds(
        Document::parse(html),
        StyleSource::Bare,
        Env {
            url: url.into(),
            user_agent: "frot-test/1".into(),
            accept_language: "en-US,en;q=0.5".into(),
        },
        &fetch,
        Deadline::on(Clock::manual(), Duration::from_millis(EXEC_CPU_MS)),
        net,
    );
    session.begin();
    between();
    run_session(&session)
}

/// A page whose script fetches `/late` — the one dispatch whose fate is the
/// whole question. A refused XHR reads back as a status-0 response the script
/// does not inspect, so the refusal shows up *only* in `stopped`: exactly the
/// signal this ball is about.
const LATE: &str = "<html><body><script>\
    var x = new XMLHttpRequest(); x.open('GET', '/late'); x.send();\
    </script></body></html>";

/// The `/late` endpoint: instant, so it costs the budget nothing of its own.
fn late(server: &mut Server) -> Mock {
    server
        .mock("GET", "/late")
        .with_status(200)
        .with_body("late")
        .create()
}

/// An endpoint that takes 400 ms *of the run's own clock* to answer — a slow
/// origin without a slow test.
fn slow(server: &mut Server, path: &str, clock: &Clock) -> Mock {
    let clock = clock.clone();
    server
        .mock("GET", path)
        .with_status(200)
        .with_chunked_body(move |w| {
            clock.advance(Duration::from_millis(400));
            w.write_all(b"globalThis.slow = 1;")
        })
        .create()
}

#[test]
fn wall_time_spent_on_compute_is_not_charged_to_the_network_budget() {
    // The defect (bl-79dc): 400 ms of the page's own compute, four times the
    // network budget, and then one subfetch. No origin was ever slow — nothing
    // was waited on at all — so the run must not report `stopped: "network"`.
    // Under a wall *window* armed once over the whole run it did exactly that.
    let mut server = Server::new();
    let _late = late(&mut server);
    let clock = Clock::manual();
    let r = run_at(LATE, &server.url(), budget(&clock), || {
        clock.advance(Duration::from_millis(400))
    });
    assert_eq!(r.stopped, None, "compute is not network (bl-79dc)");
    assert_eq!((r.scripts, r.errors, r.settled()), (1, 0, true));
}

#[test]
fn a_slow_origin_still_stops_the_run_on_the_network_bound() {
    // The other half of the same fact: when the wire really does take the whole
    // budget, the next dispatch is refused and `stopped: "network"` is true.
    let mut server = Server::new();
    let clock = Clock::manual();
    let _slow = slow(&mut server, "/slow", &clock);
    let _late = late(&mut server);
    let page = "<html><body><script>\
        var s = new XMLHttpRequest(); s.open('GET', '/slow'); s.send();\
        var x = new XMLHttpRequest(); x.open('GET', '/late'); x.send();\
        </script></body></html>";
    let r = run_at(page, &server.url(), budget(&clock), || {});
    assert_eq!(r.stopped, Some(JsStop::Network));
    assert_eq!((r.scripts, r.errors, r.settled()), (1, 0, false));
}

#[test]
fn the_concurrent_warm_wave_is_charged_too() {
    // The preload wave (bl-08f6) is the path most real network takes — the
    // initial external scripts — and it is dispatch like any other, so its
    // elapsed spends the budget. Without that charge a page whose bundles were
    // slow would find the budget untouched and never report the truncation.
    let mut server = Server::new();
    let clock = Clock::manual();
    let _slow = slow(&mut server, "/slow.js", &clock);
    let _late = late(&mut server);
    let page = format!("<html><body><script src='/slow.js'></script>{LATE}</body></html>");
    let r = run_at(&page, &server.url(), budget(&clock), || {});
    assert_eq!(r.stopped, Some(JsStop::Network));
    // The warmed bundle still ran — it was served before the budget ran out;
    // only the dispatch that followed was refused.
    assert_eq!((r.scripts, r.errors, r.settled()), (2, 0, false));
}
