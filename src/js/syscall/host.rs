//! What the syscall table binds *against* — the host-side data model.
//!
//! The shared arena, the run's static facts, and the sinks the prelude writes
//! back into, with no binding logic: [`super`] owns the functions, this owns
//! the state they close over. [`Host`] gathers all of it into one value so every
//! binding group can take the same signature (`super::GROUPS`).

use std::cell::RefCell;
use std::rc::Rc;

use super::Messages;
use crate::dom::Document;
use crate::js::geometry::SharedGeometry;
use crate::js::probe::ProbeLog;
use crate::js::subfetch::SharedSubfetch;

/// The arena, shared between the host and the syscall closures for the JS
/// phase's mutable window (js.md §2). Interior mutability, not a mirror.
pub type SharedDoc = Rc<RefCell<Document>>;

/// The static request facts the JS layer is built from: the final page URL and
/// UA string the §7 shims derive from (`navigator`/`location`), plus the caller's
/// `-H` headers, which the §6 subfetch cache rides on same-origin requests.
/// Nothing here is computed by the shim.
#[derive(Debug, Clone)]
pub struct Env {
    pub url: String,
    pub user_agent: String,
    pub headers: Vec<(String, String)>,
}

/// The counted-no-op sink (js.md §7/§11/§10): the prelude bumps it each time the
/// shim refuses a navigation it cannot honestly perform (`location` assignment).
/// The wiring layer folds it into the envelope `js.errors` count.
pub type Denials = Rc<RefCell<u32>>;

/// The reported-error sink (js.md §10): the prelude bumps it for each UNHANDLED
/// error it surfaces — `reportError`, `window.onerror`, or a dispatched window
/// `'error'` event that nothing suppresses. React >=16 *catches* render errors
/// and reports them here rather than throwing, so this is what keeps a dead app
/// from reading `errors: 0`. Folded into `js.errors`, distinct from [`Denials`]
/// (refused navigations).
pub type ReportedErrors = Rc<RefCell<u32>>;

/// The prelude's §10 reporting sinks, shared with the host: the two counts
/// folded into `js.errors` after the run — refused navigations ([`Denials`], §7/
/// §11) and reported unhandled errors ([`ReportedErrors`], §10) — plus the
/// bounded [`Messages`] detail behind them (surfaced only under `--js-errors`).
#[derive(Clone)]
pub struct Counters {
    pub denials: Denials,
    pub reported: ReportedErrors,
    pub messages: Messages,
}

/// One captured `console` call (level + rendered message), in emission order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Log {
    pub level: String,
    pub text: String,
}

/// The console sink the `__frot_console` syscall appends to.
pub type Console = Rc<RefCell<Vec<Log>>>;

/// Everything the syscall table binds against: the shared arena plus the run's
/// sinks and static facts. Bundled so every binding group takes the same three
/// arguments, which is what lets `super::GROUPS` be a uniform sequence rather
/// than seven bespoke calls.
pub struct Host {
    pub doc: SharedDoc,
    pub console: Console,
    pub geo: SharedGeometry,
    pub env: Env,
    pub counters: Counters,
    pub subfetch: SharedSubfetch,
    /// The capability-surface probe sink (`bl-bd4e`), present only when the
    /// [`super::super::probe::measure`] instrument built this host. `Some` binds
    /// the `__frot_probe` syscall and evaluates the instrumentation prelude
    /// ([`super::install`]); `None` is every shipping `--js` run — no probe
    /// syscall, no second prelude, no behaviour change.
    pub probe: Option<ProbeLog>,
}
