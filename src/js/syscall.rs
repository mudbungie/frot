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

use rquickjs::{Ctx, Exception, Function};

use super::engine::Engine;
use crate::css::{matches_any, query_all};
use crate::dom::{Document, NodeKind};

// Defined before the submodules so each group binds with the same shape.
macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

mod cookie;
mod crypto;
mod env;
mod frame;
mod host;
mod messages;
mod net;
mod subtle;

pub use host::{Console, Counters, CurrentScript, Env, Host, Log, SharedDoc};
pub use messages::{push as push_message, Message, Messages};

/// One binding group: registers its slice of the table on the realm's globals.
type Group = for<'js> fn(&Ctx<'js>, &rquickjs::Object<'js>, &Host) -> rquickjs::Result<()>;

/// The whole table, as uniform groups applied in order. Binding is all-or-
/// nothing (see [`install`]), so the sequence is a fold over one signature
/// rather than seven bespoke calls — one failure path for the whole class.
const GROUPS: &[Group] = &[
    reads,
    mutations,
    query_and_console,
    geometry,
    messages::install,
    env::install,
    net::install,
    cookie::install,
    crypto::install,
    subtle::install,
    frame::install,
];

/// Register the whole syscall table on `engine`'s realm, then evaluate the
/// prelude that builds the web-facing API on top of it. Closures capture clones
/// of the handles in `host`; nothing global.
pub fn install(engine: &Engine, host: Host) {
    // Install is host setup, not page script: every phase runs outside the page
    // bounds (js.md §5, `Engine::setup`) — the budget so a dialed-down deadline
    // can't trip mid-install (bl-5ac3), the heap cap so the prelude is never
    // parsed under starvation (bl-c385).
    engine
        .setup(|| engine.context().with(|ctx| bind(&ctx, &host)))
        .expect("install frot syscall table");
    engine
        .eval_setup(super::prelude::SOURCE)
        .expect("evaluate frot prelude");
    // The instrumentation prelude (bl-bd4e) wraps what the shipping prelude just
    // defined — `navigator`, `getContext`, the absent-global feature-detect
    // surface — so it evaluates last, and only under the measure instrument.
    if host.probe.is_some() {
        engine
            .eval_setup(super::probe::INSTRUMENT)
            .expect("evaluate frot probe instrumentation");
    }
}

/// Bind the whole table on `ctx`'s globals — the one fallible fold [`install`]
/// turns into its all-or-nothing panic. A named function, not an inline closure,
/// so the starvation sweep can exercise every allocation edge in it directly
/// (`starve_tests`) without evaluating the prelude on a starved heap.
fn bind<'js>(ctx: &Ctx<'js>, host: &Host) -> rquickjs::Result<()> {
    let g = ctx.globals();
    for group in GROUPS {
        group(ctx, &g, host)?;
    }
    // The measure instrument's probe syscall (bl-bd4e) rides the same table, but
    // only when a `ProbeLog` was supplied; a shipping `--js` run binds nothing
    // extra and evaluates no second prelude.
    if let Some(log) = &host.probe {
        let l = log.clone();
        bind!(ctx, g, "__frot_probe", move |name: String| {
            *l.borrow_mut().entry(name).or_insert(0) += 1;
        });
    }
    Ok(())
}

/// Node-query syscalls: kind/tag/attr/text and the structural links.
fn reads<'js>(ctx: &Ctx<'js>, g: &rquickjs::Object<'js>, h: &Host) -> rquickjs::Result<()> {
    let doc = &h.doc;
    // The document node's id: the receiver every document-level read and
    // write names, so the document needs no syscall of its own (`bl-196a`).
    g.set("__frot_document", Document::ROOT)?;
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
    // A doctype's [name, publicId, systemId]; `null` for any other node.
    bind!(ctx, g, "__frot_doctype", {
        let d = doc.clone();
        move |id: u32| match &d.borrow().node(id).kind {
            NodeKind::Doctype(ids) => Some(ids.to_vec()),
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
    // The executing `<script>` (js.md §4.1). A *query* over the host's one cell,
    // so `document.currentScript` cannot go stale: nothing in JS holds it, and
    // the host restores the prior value after every script (`Session::run_script`).
    bind!(ctx, g, "__frot_current_script", {
        let c = h.current.clone();
        move || c.get()
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
fn mutations<'js>(ctx: &Ctx<'js>, g: &rquickjs::Object<'js>, h: &Host) -> rquickjs::Result<()> {
    let doc = &h.doc;
    bind!(ctx, g, "__frot_create_element", {
        let d = doc.clone();
        move |name: String| d.borrow_mut().create_element(&name)
    });
    bind!(ctx, g, "__frot_create_text", {
        let d = doc.clone();
        move |text: String| d.borrow_mut().create_text(&text)
    });
    bind!(ctx, g, "__frot_create_comment", {
        let d = doc.clone();
        move |text: String| d.borrow_mut().create_comment(&text)
    });
    bind!(ctx, g, "__frot_create_fragment", {
        let d = doc.clone();
        move || d.borrow_mut().create_fragment()
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
        // Answers the reason for a refused insertion, `undefined` when done.
        move |parent: u32, child: u32, before: Option<u32>| {
            d.borrow_mut().insert_child(parent, child, before).err()
        }
    });
    // Why `child` may not take the place of `old` under `parent` — asked by
    // `replaceChild` before it removes anything.
    bind!(ctx, g, "__frot_replace_refusal", {
        let d = doc.clone();
        move |parent: u32, child: u32, old: u32| d.borrow().refusal(parent, child, None, Some(old))
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

/// `querySelector(All)` and `matches` (routed through `css::query`,
/// unsupported selectors throw) and console capture.
fn query_and_console<'js>(
    ctx: &Ctx<'js>,
    g: &rquickjs::Object<'js>,
    h: &Host,
) -> rquickjs::Result<()> {
    let (doc, console) = (&h.doc, &h.console);
    bind!(ctx, g, "__frot_query", {
        let d = doc.clone();
        move |ctx: Ctx<'js>, id: u32, sel: String| -> rquickjs::Result<Vec<u32>> {
            query_all(&d.borrow(), id, &sel).map_err(|_| bad_selector(&ctx, &sel))
        }
    });
    bind!(ctx, g, "__frot_matches", {
        let d = doc.clone();
        move |ctx: Ctx<'js>, id: u32, sel: String| -> rquickjs::Result<bool> {
            matches_any(&d.borrow(), id, &sel).map_err(|_| bad_selector(&ctx, &sel))
        }
    });
    bind!(ctx, g, "__frot_console", {
        let c = console.clone();
        move |level: String, text: String| c.borrow_mut().push(Log { level, text })
    });
    Ok(())
}

/// Geometry syscalls (js.md §8): the box read and the computed-style subset,
/// both routed through the per-generation [`SharedGeometry`] cache. Each is
/// total over any `NodeId` — a box-less node yields the all-zero rect, an
/// unknown property `""` (the totality tested in `super::geometry::tests`).
fn geometry<'js>(ctx: &Ctx<'js>, g: &rquickjs::Object<'js>, h: &Host) -> rquickjs::Result<()> {
    let (doc, geo) = (&h.doc, &h.geo);
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
        NodeKind::Document => "document",
        NodeKind::Fragment => "document-fragment",
        NodeKind::Element(_) => "element",
        NodeKind::Text(_) => "text",
        NodeKind::Comment(_) => "comment",
        NodeKind::Doctype(_) => "doctype",
    }
}

fn bad_selector(ctx: &Ctx<'_>, sel: &str) -> rquickjs::Error {
    Exception::throw_syntax(ctx, &format!("unsupported selector: {sel}"))
}

#[cfg(test)]
mod starve_tests;
#[cfg(test)]
mod tests;
