//! The script half of the mutation syscalls (js.md §4, `bl-a86b`): a
//! `<script>` that reaches the document runs, whenever it arrives — HTML's
//! "prepare the script element" on insertion, on gaining text, and on gaining a
//! `src`, exactly the three moments Firefox runs one (measured, js.md §4).
//!
//! An inline classic script runs here, synchronously, inside the call that
//! connected it — the inserting script sees its effects on the next line. An
//! external `src` or a module waits in the agent's [`Pending`] queue, tagged
//! with its window, for the run driver, which takes one after every task and
//! runs it in that window (`super::super::queue::drain_inserted`, `bl-b34e`).
//! Which scripts may run at all is [`script::prepare`]'s one answer, so a
//! parsed script, an inserted one and one a frame inserts obey the same gate.

use rquickjs::context::EvalOptions;
use rquickjs::{CatchResultExt, Ctx, Function, Object, Value};

use super::{push_message, Host};
use crate::dom::{NodeId, NodeKind};
use crate::js::script::{self, Script};

/// Register the three mutation syscalls that can make a script runnable, and
/// the cloning step that keeps a started script's copy from running.
pub fn install<'js>(ctx: &Ctx<'js>, g: &Object<'js>, h: &Host) -> rquickjs::Result<()> {
    let run = Runner::of(h);
    bind!(ctx, g, "__frot_insert_child", {
        let r = run.clone();
        // Answers the reason for a refused insertion, `undefined` when done.
        move |ctx: Ctx<'js>, parent: u32, child: u32, before: Option<u32>| {
            let moved = r.moved(child);
            let why = r.doc.borrow_mut().insert_child(parent, child, before);
            if why.is_ok() {
                // The parent's children changed; then each inserted subtree's
                // scripts, in tree order.
                let doc = r.doc.borrow();
                let mut ids = vec![parent];
                moved.iter().for_each(|&m| ids.extend(doc.scripts_in(m)));
                drop(doc);
                ids.into_iter().for_each(|id| r.prepare(&ctx, id));
            }
            why.err()
        }
    });
    bind!(ctx, g, "__frot_set_attr", {
        let r = run.clone();
        move |ctx: Ctx<'js>, id: u32, name: String, value: String| {
            r.doc.borrow_mut().set_attr(id, &name, &value);
            if name.eq_ignore_ascii_case("src") {
                r.prepare(&ctx, id);
            }
        }
    });
    bind!(ctx, g, "__frot_set_text", {
        let r = run.clone();
        move |ctx: Ctx<'js>, id: u32, text: String| {
            r.doc.borrow_mut().set_text(id, &text);
            r.prepare(&ctx, id);
        }
    });
    // HTML's cloning steps for a script: the copy is started if the original
    // was, so a clone of a script that ran never runs (measured in Firefox).
    // A script's "force async" flag (`bl-f9f2`): read by `async`'s getter,
    // unset by its setter.
    bind!(ctx, g, "__frot_force_async", {
        let d = h.doc.clone();
        move |id: u32| d.borrow().force_async(id)
    });
    bind!(ctx, g, "__frot_unforce", {
        let d = h.doc.clone();
        move |id: u32| d.borrow_mut().unforce(id)
    });
    bind!(ctx, g, "__frot_clone_started", {
        let d = h.doc.clone();
        move |from: u32, to: u32| {
            let mut doc = d.borrow_mut();
            if doc.started(from) {
                doc.start(to);
            }
        }
    });
    Ok(())
}

/// The handles the hook needs, shared by the three bindings.
#[derive(Clone)]
struct Runner {
    doc: super::SharedDoc,
    counters: super::Counters,
    current: super::CurrentScript,
    pending: super::Pending,
    location: String,
    realm: Option<usize>,
}

impl Runner {
    fn of(h: &Host) -> Self {
        Runner {
            doc: h.doc.clone(),
            counters: h.counters.clone(),
            current: h.current.clone(),
            pending: h.pending.clone(),
            location: h.location.clone(),
            realm: h.realm,
        }
    }

    /// What an insertion of `child` moves: a fragment's children, or itself.
    fn moved(&self, child: NodeId) -> Vec<NodeId> {
        let doc = self.doc.borrow();
        match doc.node(child).kind {
            NodeKind::Fragment => doc.node(child).children.clone(),
            _ => vec![child],
        }
    }

    /// Prepare `id`: run it now if it is an inline classic script, queue it if
    /// it is external or a module, and do nothing if it may not run.
    fn prepare(&self, ctx: &Ctx<'_>, id: NodeId) {
        let h = self;
        let prepared = script::prepare(&mut h.doc.borrow_mut(), id, &h.location, false);
        match prepared {
            Some(Script::Inline {
                module: false,
                body,
            }) => self.run(ctx, id, &body),
            Some(s) => {
                let window = h.realm.map(|realm| super::Window {
                    realm,
                    current: h.current.clone(),
                });
                h.pending.borrow_mut().push_back((id, s, window));
            }
            None => {}
        }
    }

    /// Run one inline classic script inside the call that inserted it: sloppy
    /// global code, `id` its `currentScript`. A throw is the script's, never
    /// the inserting call's — counted, captured, and swallowed, as Firefox
    /// reports it and carries on. No microtask checkpoint: the inserting
    /// script is still on the stack (HTML "clean up after running script").
    /// A budget trip mid-script surfaces here as a throw too, and the
    /// interrupt fires again in the inserting script, which stops the run.
    fn run(&self, ctx: &Ctx<'_>, id: NodeId, body: &str) {
        let h = self;
        let mut opts = EvalOptions::default();
        opts.global = true;
        opts.strict = false;
        opts.backtrace_barrier = false;
        opts.promise = false;
        let prior = h.current.replace(Some(id));
        let out = ctx
            .eval_with_options::<Value, _>(body, opts)
            .catch(ctx)
            .map_err(|e| e.to_string());
        h.current.set(prior);
        h.counters.scripts.set(h.counters.scripts.get() + 1);
        if let Err(msg) = out {
            h.counters.threw.set(h.counters.threw.get() + 1);
            push_message(&h.counters.messages, "throw", &msg);
        }
    }
}
