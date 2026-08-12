//! Negative controls for the golden suite's §5 harness (`bl-c81a`).
//!
//! Every golden asserts `settled: true`, which is only evidence if the same
//! harness can still report `settled: false`. `golden_tests::golden_bounds`
//! hands the network window to the test on a frozen [`Clock::manual`]; these
//! prove that handing it over did not disarm it — a network window driven to
//! expiry still stops the run on `"network"`, and the compute window, which
//! stays on the real CPU clock, still stops it on `"budget"`.
//!
//! Both bounds are dialled by the *test*, not by the host: the network window
//! is spent on a clock only this file advances, and the compute window is
//! dialled down against a real [`Clock::cpu`], which a spinning script burns at
//! the same rate on an idle laptop and a saturated box. Nothing here can be
//! made to pass or fail by machine load — the property the goldens lost.

use super::golden_tests::{capture_bounded, env, serve};
use crate::js::engine::{Clock, Deadline, NET_BUDGET_MS};
use crate::js::Bounds;
use std::time::Duration;

/// A shell whose only content arrives from an external script — the §6
/// subfetch the network window governs.
const EXTERNAL_PAGE: &str = "<html><body><div id='root'></div>\
    <script src='/app.js'></script></body></html>";
const EXTERNAL_APP: &str = "document.getElementById('root').textContent='rendered';";

/// A shell whose inline script never returns, so the compute window is what
/// ends the run — no subfetch involved.
const SPIN_PAGE: &str = "<html><body><div id='root'></div>\
    <script>while(true){}</script></body></html>";

#[test]
fn an_expired_network_window_still_reports_stopped_network() {
    let (_s, url) = serve(EXTERNAL_PAGE, &[("/app.js", EXTERNAL_APP)]);
    // A zero-length window on the frozen clock: armed at the run's start, it is
    // already past, so the §6 seam refuses to dispatch. This is the goldens'
    // own clock driven to expiry — the bound is under test control, not gone.
    let bounds = Bounds {
        cpu: Deadline::compute(),
        net: Deadline::on(Clock::manual(), Duration::ZERO),
    };
    let (code, out) = capture_bounded(&[&url, "--js", "--out", "text"], bounds);
    assert_eq!(code, 0);
    let js = &env(&out)["js"];
    assert_eq!(js["settled"], false, "{js}");
    assert_eq!(js["stopped"], "network", "{js}");
    // The refused script never ran, and §4.2 counts the skip like a failed
    // stylesheet — so the shell is still empty and honestly reads needs-js.
    assert_eq!(
        (js["scripts"].as_u64(), js["errors"].as_u64()),
        (Some(0), Some(1))
    );
    assert_eq!(env(&out)["status"], "needs");
}

#[test]
fn a_spinning_script_still_reports_stopped_budget() {
    let (_s, url) = serve(SPIN_PAGE, &[]);
    // The compute window dialled down against the real CPU clock (the 4.1
    // spike's short-budget pattern), so the trip is fast and deterministic.
    let bounds = Bounds {
        cpu: Deadline::on(Clock::cpu(), Duration::from_millis(20)),
        net: Deadline::on(Clock::manual(), Duration::from_millis(NET_BUDGET_MS)),
    };
    let (code, out) = capture_bounded(&[&url, "--js", "--out", "text"], bounds);
    assert_eq!(code, 0);
    let js = &env(&out)["js"];
    assert_eq!(js["settled"], false, "{js}");
    assert_eq!(js["stopped"], "budget", "{js}");
}
