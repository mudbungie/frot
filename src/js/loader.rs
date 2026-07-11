//! ES module resolver + loader over the §6 subfetch cache (js.md §4.1/§6).
//!
//! `type="module"` scripts and their `import` graph resolve and load through the
//! *same* once-then-frozen [`SharedSubfetch`] that `fetch`/XHR and external
//! `<script src>` ride — GET-only, resolved-URL cache key, same-origin header
//! rules, `SUBFETCH_MAX` cap, remote→local refused. No side channel: a module's
//! bytes come from exactly one subfetch, counted against the one cap.
//!
//! Installed on the engine's runtime via rquickjs's `loader` feature
//! ([`Runtime::set_loader`], gated behind that feature — the sole reason the
//! `relative-path` dep rides in; see js.md §1 decision log). No `rquickjs` type
//! escapes `src/js/` (js.md §1): these trait impls live here.

use rquickjs::loader::{ImportAttributes, Loader, Resolver};
use rquickjs::module::Declared;
use rquickjs::{Ctx, Error, Module, Result};
use url::Url;

use super::engine::Engine;
use super::subfetch::{Outcome, SharedSubfetch};

/// Wire the module loader onto `engine`, resolving and loading through the shared
/// §6 cache. Called once at [`super::Session`] construction, before any module
/// evaluates.
pub fn install(engine: &Engine, subfetch: &SharedSubfetch) {
    engine.set_loader(SubfetchResolver, SubfetchLoader(subfetch.clone()));
}

/// Resolve a module specifier against the importing module's URL (js.md §4.1/§6).
/// A relative reference (`/`, `./`, `../`) or an absolute-URL specifier joins
/// against `base` (the entry module's page URL, or a dependency's own resolved
/// URL — so nested imports chain correctly). A *bare* specifier has no import map
/// here, so it is unresolvable exactly as in a browser without one, and fails as
/// a counted §10 error rather than being silently guessed at.
struct SubfetchResolver;

impl Resolver for SubfetchResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> Result<String> {
        if is_bare(name) {
            return Err(Error::new_resolving_message(
                base,
                name,
                "bare module specifiers are unsupported (no import map)",
            ));
        }
        Url::parse(base)
            .and_then(|b| b.join(name))
            .map(|u| u.to_string())
            .map_err(|_| Error::new_resolving(base, name))
    }
}

/// A bare specifier is neither a relative reference nor an absolute URL — a
/// browser rejects it without an import map (js.md §4.1).
fn is_bare(name: &str) -> bool {
    !(name.starts_with('/')
        || name.starts_with("./")
        || name.starts_with("../")
        || Url::parse(name).is_ok())
}

/// Load a resolved module URL's source through the once-then-frozen §6 cache,
/// declaring it under its own URL so nested imports resolve against it. A refused
/// or transport-failed fetch, or a non-2xx response, is a load error — surfaced
/// through the §5/§10 unified channel (the importing `import` throws, counted),
/// with sibling scripts continuing.
struct SubfetchLoader(SharedSubfetch);

impl Loader for SubfetchLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> Result<Module<'js, Declared>> {
        // Bind the outcome so the cache borrow is released before `Module::declare`
        // recurses into this loader for the module's own imports (nested graphs).
        let outcome = self.0.borrow_mut().get(name);
        match outcome {
            Outcome::Got(f) if f.ok => Module::declare(ctx.clone(), name, f.body),
            Outcome::Got(_) => Err(Error::new_loading_message(name, "module fetch was not ok")),
            Outcome::Failed(reason) => Err(Error::new_loading_message(name, reason)),
        }
    }
}

#[cfg(test)]
mod tests;
