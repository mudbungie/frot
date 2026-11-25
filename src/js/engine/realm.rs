//! Second realms — the engine half of an iframe's window (js.md §14).
//!
//! A browser gives every nested browsing context its own global and its own
//! intrinsics (`frame.contentWindow.Array !== Array`), all on one agent: one
//! heap, one job queue, one event loop. quickjs has exactly that shape — many
//! `JSContext`s on one `JSRuntime` — so a frame's window is a second context on
//! the engine's runtime. The heap cap, the compute interrupt and the microtask
//! queue are runtime-wide, so a frame spends the run's one budget by
//! construction (js.md §5).
//!
//! A frame is opened from *inside* a syscall — a page reads `contentWindow` —
//! where the runtime is already locked, and rquickjs's safe constructors lock it
//! again. So this is the one place frot creates a context through the C API.
//! Every context opened here is owned by [`Realms`] and closed by
//! [`Realms::close_all`] before the runtime is freed (`Engine`'s `Drop`): a
//! context held only by a closure inside the runtime would be a cycle no
//! collector can see, and quickjs aborts on a leaked object at teardown.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::rc::Rc;

use rquickjs::context::EvalOptions;
use rquickjs::runtime::WeakRuntime;
use rquickjs::{qjs, Context, Ctx};

/// The frame contexts one engine has opened, owned until the engine drops.
/// Cloning shares the one list, so the syscall that opens a frame and the
/// engine that closes them hold one authority.
#[derive(Clone)]
pub struct Realms {
    rt: WeakRuntime,
    mem_limit: usize,
    owned: Rc<RefCell<Vec<Context>>>,
}

impl Realms {
    pub(super) fn new(rt: WeakRuntime, mem_limit: usize) -> Self {
        Realms {
            rt,
            mem_limit,
            owned: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Open a fresh realm — a context with every standard intrinsic — on the
    /// runtime `ctx` runs on, from inside a call that holds the runtime lock.
    /// Its global is a new object with its own `Array`, `Function`, `eval`.
    ///
    /// The intrinsics are built with the heap cap lifted, as the prelude is
    /// parsed (`eval_setup`): quickjs-ng's `JS_NewContext` is not
    /// allocation-failure-safe either — a failure part-way through the
    /// intrinsics frees a half-built context whose GC header is garbage, and
    /// the next collection `abort()`s the process (found by the starvation
    /// sweep in `syscall::frame`). They are the engine's fixed set, never page
    /// input, so only a real `malloc` failure can refuse them.
    pub fn open<'js>(&self, ctx: &Ctx<'js>) -> Ctx<'js> {
        // SAFETY: `ctx` is live and its runtime is locked for the length of this
        // call (we are inside it), which is what `JS_NewContext` requires. The
        // new context's one owning reference moves into `owned`; the `Ctx`
        // handed back takes its own (`from_raw` dups) and shares `ctx`'s
        // lifetime, so it cannot outlive the lock either.
        let raw = self.uncapped(ctx, || unsafe {
            qjs::JS_NewContext(qjs::JS_GetRuntime(ctx.as_raw().as_ptr()))
        });
        let raw = NonNull::new(raw).expect("quickjs frame context");
        let owner = self
            .rt
            .try_ref()
            .expect("a realm opens while its runtime runs");
        unsafe {
            self.owned.borrow_mut().push(Context::from_raw(raw, owner));
            Ctx::from_raw(raw)
        }
    }

    /// Run `f` with the heap cap lifted, restoring the engine's cap after.
    fn uncapped<T>(&self, ctx: &Ctx<'_>, f: impl FnOnce() -> T) -> T {
        // SAFETY: the runtime is locked (we are inside a call on `ctx`), and a
        // memory limit is plain runtime state; 0 is quickjs's "unlimited".
        let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
        unsafe { qjs::JS_SetMemoryLimit(rt, 0) };
        let out = f();
        unsafe { qjs::JS_SetMemoryLimit(rt, self.mem_limit as _) };
        out
    }

    /// Evaluate frot's own setup source (the prelude) in `ctx` — strict, global
    /// code — with the heap cap lifted for the parse and restored after. Not the
    /// compute budget: a frame is opened by a page, so its install is the page's
    /// work and is charged to it (js.md §14). The heap half is `Engine::setup`'s
    /// reason (bl-c385): quickjs's parser is not allocation-failure-safe, and the
    /// prelude is frot's fixed program, never page input.
    pub fn eval_setup(&self, ctx: &Ctx<'_>, src: &str) -> rquickjs::Result<()> {
        let mut opts = EvalOptions::default();
        opts.global = true;
        opts.strict = true;
        opts.promise = false;
        self.uncapped(ctx, || ctx.eval_with_options::<(), _>(src, opts))
    }

    /// The index the next [`open`](Self::open)ed realm will have (`bl-b34e`).
    pub fn next_index(&self) -> usize {
        self.owned.borrow().len()
    }

    /// The realm opened `i`th, to evaluate in from outside any call.
    pub(super) fn context(&self, i: usize) -> Context {
        self.owned.borrow()[i].clone()
    }

    /// Close every frame context (the engine's teardown, before its runtime).
    pub(super) fn close_all(&self) {
        self.owned.borrow_mut().clear();
    }
}
