//! ES-module evaluation and its §10 rejection accounting (js.md §4.1/§6).
//!
//! The other half of [`Engine`]'s eval surface, split out under the 300-line
//! cap: a module is not a script — it links through the installed loader, it is
//! strict by definition (no mode to choose, unlike a classic script — see
//! [`Engine::eval_script`]), and it *settles* rather than returning, so its
//! outcome is a promise the §10 unhandled-rejection net has to account for.

use std::sync::atomic::Ordering;

use rquickjs::function::This;
use rquickjs::promise::{Promise, PromiseState};
use rquickjs::{CatchResultExt, Ctx, Function, Module, Value};

use super::{Engine, EvalError};

impl Engine {
    /// Evaluate `src` as an ES module named `name` (its URL, the base every
    /// `import` resolves against, js.md §4.1/§6) inside the current budget
    /// window, then drain microtasks. The whole `import` graph loads synchronously
    /// through the installed loader ([`Engine::set_loader`]); an unresolvable
    /// specifier or failed module fetch throws here and surfaces as
    /// [`EvalError::Exception`] (a counted §10 error), sibling scripts continuing.
    /// Module evaluation is async by spec — the returned promise settles as its
    /// top-level await / dynamic `import()` drain here and, for timer-bound waits,
    /// across the §5 settle loop; a late unhandled rejection is caught by the §10
    /// net ([`Engine::rejections`]). A budget trip anywhere is [`EvalError::Budget`].
    pub fn eval_module(&self, name: &str, src: &str) -> Result<String, EvalError> {
        let before = self.rejections.get();
        let res = self.ctx.with(|ctx| -> Result<(), String> {
            let promise = Module::evaluate(ctx.clone(), name.to_string(), src.to_string())
                // A synchronous hard failure — syntax error, an unresolvable
                // specifier, or a failed module fetch (the loader threw) — reaches
                // here before a promise exists; counted once (js.md §4.1/§10).
                .catch(&ctx)
                .map_err(|e| e.to_string())?;
            self.watch_rejection(&ctx, &promise);
            // A module that throws *synchronously* at top level is already rejected
            // here, and quickjs may report that rejection to the tracker more than
            // once. The watcher owns this promise's outcome, so undo the tracker's
            // report for it (nothing else ran between `before` and now) — the watcher
            // counts it exactly once when its reject microtask drains.
            if promise.state() == PromiseState::Rejected {
                self.rejections.set(before);
            }
            Ok(())
        });
        self.drain_jobs();
        if self.tripped.load(Ordering::Relaxed) {
            Err(EvalError::Budget)
        } else {
            res.map(|()| String::new()).map_err(EvalError::Exception)
        }
    }

    /// Attach a rejection watcher to a module evaluation promise (js.md §10). As a
    /// handler it keeps the promise's *own* rejection from double-reporting through
    /// the tracker, and its reject arm counts that rejection once via the same net —
    /// so a top-level throw or a rejected top-level await counts as one, whenever it
    /// settles (inline, or later across the §5 loop). A `.catch` a page attaches
    /// itself still nets out through the tracker as before.
    fn watch_rejection<'js>(&self, ctx: &Ctx<'js>, promise: &Promise<'js>) {
        let rej = self.rejections.clone();
        let on_reject = Function::new(ctx.clone(), move |_v: Value<'js>| {
            rej.set(rej.get() + 1);
        })
        .expect("module rejection watcher");
        let noop = Function::new(ctx.clone(), || {}).expect("module settle noop");
        promise
            .then()
            .and_then(|t| t.call::<_, ()>((This(promise.clone()), noop, on_reject)))
            .expect("attach module rejection watcher");
    }

    /// Net unhandled promise rejections observed so far (§10). A late `.catch`
    /// un-counts an earlier report, so the net never ends negative in practice;
    /// it is clamped at zero and reported as a `u32`.
    pub fn rejections(&self) -> u32 {
        self.rejections.get().max(0) as u32
    }
}
