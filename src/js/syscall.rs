//! The narrow DOM syscall table (js.md §3) — the **entire** Rust↔JS surface.
//!
//! Page scripts never touch the arena directly; they hold opaque integer
//! handles ([`NodeId`]s) and reach the one document through this fixed table of
//! host functions. Breadth lives in the JS prelude ([`super::prelude`]) layered
//! over these calls, so the shim grows without widening the Rust interface.
//!
//! Every handle a script sees was minted by a syscall, so ids are always in
//! range — the table trusts them exactly as every other arena consumer does
//! (`cascade`, the views), never bounds-checking a `NodeId`.

use std::cell::RefCell;
use std::rc::Rc;

use rquickjs::{Ctx, Exception, Function};

use super::engine::Engine;
use super::geometry::SharedGeometry;
use super::subfetch::SharedSubfetch;
use crate::css::query_all;
use crate::dom::{Document, NodeKind};

mod env;
mod messages;
mod net;

pub use messages::{push as push_message, Message, Messages};

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
/// Bundled so the [`install`] surface stays narrow.
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

/// Register the whole syscall table on `engine`'s realm, then evaluate the
/// prelude that builds the web-facing API on top of it. Closures capture clones
/// of `doc`/`console`; nothing global.
pub fn install(
    engine: &Engine,
    doc: SharedDoc,
    console: Console,
    geo: SharedGeometry,
    env: Env,
    counters: Counters,
    subfetch: SharedSubfetch,
) {
    engine
        .context()
        .with(|ctx| -> rquickjs::Result<()> {
            let g = ctx.globals();
            reads(&ctx, &g, &doc)?;
            mutations(&ctx, &g, &doc)?;
            query_and_console(&ctx, &g, &doc, &console)?;
            geometry(&ctx, &g, &doc, &geo)?;
            reported_errors(&ctx, &g, &counters.reported, &counters.messages)?;
            env::install(&ctx, &g, env, &counters.denials)?;
            net::install(&ctx, &g, &subfetch)?;
            Ok(())
        })
        .expect("install frot syscall table");
    // Prelude install is host setup, not page script: evaluate it exempt from
    // the page budget (js.md §5) so a dialed-down budget can't trip mid-install
    // (bl-5ac3). Page scripts arm their own deadline via `Session::begin`.
    engine
        .eval_setup(super::prelude::SOURCE)
        .expect("evaluate frot prelude");
}

macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

/// Node-query syscalls: kind/tag/attr/text and the structural links.
fn reads<'js>(ctx: &Ctx<'js>, g: &rquickjs::Object<'js>, doc: &SharedDoc) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_roots", {
        let d = doc.clone();
        move || d.borrow().roots().to_vec()
    });
    bind!(ctx, g, "__frot_kind", {
        let d = doc.clone();
        move |id: u32| kind_label(&d.borrow(), id)
    });
    bind!(ctx, g, "__frot_tag", {
        let d = doc.clone();
        move |id: u32| match &d.borrow().node(id).kind {
            NodeKind::Element(e) => Some(e.name.clone()),
            _ => None,
        }
    });
    bind!(ctx, g, "__frot_attr", {
        let d = doc.clone();
        move |id: u32, name: String| match &d.borrow().node(id).kind {
            NodeKind::Element(e) => e.attr(&name).map(str::to_string),
            _ => None,
        }
    });
    bind!(ctx, g, "__frot_attrs", {
        let d = doc.clone();
        move |id: u32| match &d.borrow().node(id).kind {
            NodeKind::Element(e) => e
                .attrs
                .iter()
                .map(|a| vec![a.name.clone(), a.value.clone()])
                .collect(),
            _ => Vec::<Vec<String>>::new(),
        }
    });
    bind!(ctx, g, "__frot_text", {
        let d = doc.clone();
        move |id: u32| d.borrow().text_content(id)
    });
    bind!(ctx, g, "__frot_parent", {
        let d = doc.clone();
        move |id: u32| d.borrow().node(id).parent
    });
    bind!(ctx, g, "__frot_children", {
        let d = doc.clone();
        move |id: u32| d.borrow().node(id).children.clone()
    });
    Ok(())
}

/// The §2 mutation syscalls plus fragment parsing (the `innerHTML` primitive).
fn mutations<'js>(
    ctx: &Ctx<'js>,
    g: &rquickjs::Object<'js>,
    doc: &SharedDoc,
) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_create_element", {
        let d = doc.clone();
        move |name: String| d.borrow_mut().create_element(&name)
    });
    bind!(ctx, g, "__frot_create_text", {
        let d = doc.clone();
        move |text: String| d.borrow_mut().create_text(&text)
    });
    bind!(ctx, g, "__frot_set_attr", {
        let d = doc.clone();
        move |id: u32, name: String, value: String| d.borrow_mut().set_attr(id, &name, &value)
    });
    bind!(ctx, g, "__frot_remove_attr", {
        let d = doc.clone();
        move |id: u32, name: String| d.borrow_mut().remove_attr(id, &name)
    });
    bind!(ctx, g, "__frot_set_text", {
        let d = doc.clone();
        move |id: u32, text: String| d.borrow_mut().set_text(id, &text)
    });
    bind!(ctx, g, "__frot_insert_child", {
        let d = doc.clone();
        move |parent: u32, child: u32, index: u32| {
            d.borrow_mut().insert_child(parent, child, index as usize)
        }
    });
    bind!(ctx, g, "__frot_detach", {
        let d = doc.clone();
        move |id: u32| d.borrow_mut().detach(id)
    });
    bind!(ctx, g, "__frot_fragment", {
        let d = doc.clone();
        move |html: String| d.borrow_mut().parse_fragment(&html)
    });
    Ok(())
}

/// `querySelector(All)` (routed through `css::query`, unsupported selectors
/// throw) and console capture.
fn query_and_console<'js>(
    ctx: &Ctx<'js>,
    g: &rquickjs::Object<'js>,
    doc: &SharedDoc,
    console: &Console,
) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_query", {
        let d = doc.clone();
        move |ctx: Ctx<'js>, id: u32, sel: String| -> rquickjs::Result<Vec<u32>> {
            query_all(&d.borrow(), Some(id), &sel).map_err(|_| bad_selector(&ctx, &sel))
        }
    });
    bind!(ctx, g, "__frot_query_doc", {
        let d = doc.clone();
        move |ctx: Ctx<'js>, sel: String| -> rquickjs::Result<Vec<u32>> {
            query_all(&d.borrow(), None, &sel).map_err(|_| bad_selector(&ctx, &sel))
        }
    });
    bind!(ctx, g, "__frot_console", {
        let c = console.clone();
        move |level: String, text: String| c.borrow_mut().push(Log { level, text })
    });
    Ok(())
}

/// The §10 reported-error syscall: `__frot_report_error(text)` bumps the shared
/// counter for each unhandled error the prelude surfaces (`reportError` /
/// `window.onerror` / a dispatched window `'error'` event) and captures its
/// message into the bounded [`Messages`] sink. Distinct from the §7 navigation
/// [`Denials`] — this is the caught-and-reported app-failure channel React et al.
/// route through instead of throwing.
fn reported_errors<'js>(
    ctx: &Ctx<'js>,
    g: &rquickjs::Object<'js>,
    reported: &ReportedErrors,
    messages: &Messages,
) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_report_error", {
        let r = reported.clone();
        let m = messages.clone();
        move |text: String| {
            *r.borrow_mut() += 1;
            messages::push(&m, "report", &text);
        }
    });
    Ok(())
}

/// Geometry syscalls (js.md §8): the box read and the computed-style subset,
/// both routed through the per-generation [`SharedGeometry`] cache. Each is
/// total over any `NodeId` — a box-less node yields the all-zero rect, an
/// unknown property `""` (the totality tested in `super::geometry::tests`).
fn geometry<'js>(
    ctx: &Ctx<'js>,
    g: &rquickjs::Object<'js>,
    doc: &SharedDoc,
    geo: &SharedGeometry,
) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_rect", {
        let (d, ge) = (doc.clone(), geo.clone());
        move |id: u32| ge.borrow_mut().rect(&d.borrow(), id)
    });
    bind!(ctx, g, "__frot_computed_style", {
        let (d, ge) = (doc.clone(), geo.clone());
        move |id: u32, prop: String| ge.borrow_mut().computed(&d.borrow(), id, &prop)
    });
    Ok(())
}

fn kind_label(doc: &Document, id: u32) -> &'static str {
    match doc.node(id).kind {
        NodeKind::Element(_) => "element",
        NodeKind::Text(_) => "text",
        NodeKind::Comment(_) => "comment",
        NodeKind::Doctype => "doctype",
    }
}

fn bad_selector(ctx: &Ctx<'_>, sel: &str) -> rquickjs::Error {
    Exception::throw_syntax(ctx, &format!("unsupported selector: {sel}"))
}

#[cfg(test)]
mod starve_tests;
#[cfg(test)]
mod tests;
