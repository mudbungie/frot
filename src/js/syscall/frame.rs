//! The frame syscall (js.md §14): `__frot_open_frame(agent)` builds an iframe's
//! window — a second realm with its own `about:blank` document — and returns its
//! global.
//!
//! The new window is a whole browser global, not a bare set of intrinsics: the
//! same syscall table and the same prelude as the top window, bound to a
//! document of its own. What it shares with its creator is what a same-origin
//! `about:blank` shares in a browser: the origin and its cookies (`env`), the
//! network and its budget, the console and the §10 error sinks. The `agent` is
//! the creator's event-loop state (`loop.js`), so every window schedules into
//! one queue on one clock, as one agent's windows do. The JS side (`frame.js`)
//! decides *when* a window is opened and caches it; this side only builds one.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rquickjs::{Ctx, Function, Object, Value};

use super::{bind, Env, Host};
use crate::dom::Document;
use crate::js::geometry::{Geometry, StyleSource};

/// The address of an iframe's initial document (HTML "create a new child
/// navigable"): what its `location` reports until a navigation it never makes.
const INITIAL: &str = "about:blank";

/// Register `__frot_open_frame` on the realm.
pub fn install<'js>(ctx: &Ctx<'js>, g: &Object<'js>, host: &Host) -> rquickjs::Result<()> {
    let creator = Creator {
        console: host.console.clone(),
        env: host.env.clone(),
        counters: host.counters.clone(),
        subfetch: host.subfetch.clone(),
        cookie: host.cookie.clone(),
        clock: host.clock.clone(),
        realms: host.realms.clone(),
        pending: host.pending.clone(),
    };
    let open = move |ctx: Ctx<'js>, agent: Value<'js>| open(&ctx, creator.host(), agent);
    g.set(
        "__frot_open_frame",
        Function::new(ctx.clone(), open)?.with_name("__frot_open_frame")?,
    )
}

/// What a frame inherits from the window that opened it.
struct Creator {
    console: super::Console,
    env: super::Env,
    counters: super::Counters,
    subfetch: crate::js::subfetch::SharedSubfetch,
    cookie: crate::fetch::SharedJar,
    clock: crate::js::engine::Clock,
    realms: crate::js::engine::Realms,
    pending: super::Pending,
}

impl Creator {
    /// A host for a new frame: the shared sinks and facts, plus the frame's own
    /// document (`about:blank` — html5ever's `html`/`head`/`body` skeleton, which
    /// is exactly Firefox's initial frame document), geometry, current-script
    /// cell and address. No probe: the measure instrument watches the top
    /// window only.
    fn host(&self) -> Host {
        Host {
            doc: Rc::new(RefCell::new(Document::parse(""))),
            console: self.console.clone(),
            geo: Rc::new(RefCell::new(Geometry::new(StyleSource::Bare))),
            env: Env {
                referrer: String::new(),
                ..self.env.clone()
            },
            counters: self.counters.clone(),
            subfetch: self.subfetch.clone(),
            cookie: self.cookie.clone(),
            clock: self.clock.clone(),
            probe: None,
            current: Rc::new(Cell::new(None)),
            // The agent's one queue, tagged with the realm about to open.
            pending: self.pending.clone(),
            realm: Some(self.realms.next_index()),
            location: INITIAL.into(),
            realms: self.realms.clone(),
        }
    }
}

/// Open the realm, bind the table against `host`, hand it the creator's event
/// loop, and evaluate the prelude — every step's failure (an allocation the heap
/// cap refuses while binding, the compute budget tripping mid-prelude) thrown to
/// the page.
fn open<'js>(ctx: &Ctx<'js>, host: Host, agent: Value<'js>) -> rquickjs::Result<Object<'js>> {
    let frame = host.realms.open(ctx);
    let g = frame.globals();
    // One fallible step for the table and the hand-off: the sweep in `tests`
    // starves them as one.
    bind(&frame, &host).and_then(|()| g.set("__frot_agent", agent))?;
    host.realms.eval_setup(&frame, crate::js::prelude::SOURCE)?;
    Ok(g)
}

#[cfg(test)]
mod tests;
