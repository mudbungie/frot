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
use crate::fetch::SharedJar;
use crate::js::engine::Clock;
use crate::js::geometry::SharedGeometry;
use crate::js::probe::ProbeLog;
use crate::js::subfetch::SharedSubfetch;

/// The arena, shared between the host and the syscall closures for the JS
/// phase's mutable window (js.md §2). Interior mutability, not a mirror.
pub type SharedDoc = Rc<RefCell<Document>>;

/// The static request facts the JS layer is built from: the final page URL and
/// UA string the §7 shims derive from (`navigator`/`location`). The caller's
/// `-H` headers live on the shared [`crate::fetch::FetchSession`] the §6 subfetch
/// cache dispatches through, not here. Nothing here is computed by the shim.
#[derive(Debug, Clone)]
pub struct Env {
    pub url: String,
    pub user_agent: String,
    /// The effective `Accept-Language` (default persona value, or the caller's
    /// `-H` override — [`crate::fetch::accept_language`]). `navigator.language`/
    /// `.languages` derive from *this* string (`bl-3972`, identity.md §8), the
    /// same source as the HTTP header, so the two can never disagree.
    pub accept_language: String,
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
    /// The session's one monotonic [`Clock`] (`bl-e707`): the `__frot_now` syscall
    /// reads real elapsed off it so `performance.now`/`Date.now` advance with
    /// actual CPU/host/network time, the same clock the §5/§6 deadline bounds the
    /// run with — one authority, a virtual offset (in JS) only for timer jumps.
    pub clock: Clock,
    /// The shared cookie jar (bl-6dad): the `document.cookie` syscalls read/write
    /// the *same* jar the transport does, at `env.url` (the final document). One
    /// authority, so a `Set-Cookie` from the document GET is visible to JS (iff
    /// non-HttpOnly) and a JS write feeds a later same-origin GET.
    pub cookie: SharedJar,
    /// The capability-surface probe sink (`bl-bd4e`), present only when the
    /// [`super::super::probe::measure`] instrument built this host. `Some` binds
    /// the `__frot_probe` syscall and evaluates the instrumentation prelude
    /// ([`super::install`]); `None` is every shipping `--js` run — no probe
    /// syscall, no second prelude, no behaviour change.
    pub probe: Option<ProbeLog>,
}
