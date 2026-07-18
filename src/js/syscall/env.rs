//! Environment syscalls (js.md §7) — the static facts the JS `env.js` prelude
//! needs to build `navigator`/`location`/`matchMedia`, plus the counted-no-op
//! sink for navigation the shim refuses (assigning `location`, §11).
//!
//! URL decomposition is done here, by the `url` crate, so the parser stays the
//! single source of truth for URL semantics; the prelude never re-parses.

use rquickjs::{Ctx, Function, Object};
use url::Url;

use super::{Denials, Env};
use crate::layout::VIEWPORT_WIDTH;

/// Bind one named host function, keeping the `?` error paths on the call line so
/// line coverage sees the executed (never-erroring) `set` — the same shape the
/// core table's `bind!` macro uses.
macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

/// Register `__frot_env_ua`, `__frot_location`, `__frot_viewport_width`, and
/// `__frot_denied` on the realm. Consumes `env`, moving its facts into the
/// closures; `denials` is the shared counter the prelude bumps on a refused
/// navigation.
pub fn install<'js>(
    ctx: &Ctx<'js>,
    g: &Object<'js>,
    env: Env,
    denials: &Denials,
) -> rquickjs::Result<()> {
    let ua = env.user_agent;
    bind!(ctx, g, "__frot_env_ua", move || ua.clone());
    let url = env.url;
    bind!(ctx, g, "__frot_location", move |ctx: Ctx<'js>| location_obj(&ctx, &url));
    bind!(ctx, g, "__frot_url_parse", |ctx: Ctx<'js>, spec: String, base: Option<String>| {
        url_parse(&ctx, &spec, base.as_deref())
    });
    bind!(ctx, g, "__frot_viewport_width", || VIEWPORT_WIDTH);
    let d = denials.clone();
    bind!(ctx, g, "__frot_denied", move || *d.borrow_mut() += 1);
    Ok(())
}

/// Decompose a parsed [`Url`] into the standard read-only members. The single
/// source of URL semantics for both `location` and `new URL` (js.md §7) — the
/// prelude never re-parses.
fn decompose<'js>(obj: &Object<'js>, u: &Url) -> rquickjs::Result<()> {
    let host = u.host_str().unwrap_or("");
    let hostport = match u.port() {
        Some(p) => format!("{host}:{p}"),
        None => host.to_string(),
    };
    obj.set("href", u.as_str())?;
    obj.set("protocol", format!("{}:", u.scheme()))?;
    obj.set("host", hostport)?;
    obj.set("hostname", host)?;
    obj.set("port", u.port().map(|p| p.to_string()).unwrap_or_default())?;
    obj.set("pathname", u.path())?;
    obj.set("search", u.query().map(|q| format!("?{q}")).unwrap_or_default())?;
    obj.set("hash", u.fragment().map(|f| format!("#{f}")).unwrap_or_default())?;
    obj.set("origin", u.origin().ascii_serialization())?;
    Ok(())
}

/// Build the `location` object. A parseable URL is decomposed into the standard
/// read-only members; an unparseable one still yields `href` (empty components),
/// never a throw — `location` is a fact, not a denial.
fn location_obj<'js>(ctx: &Ctx<'js>, raw: &str) -> rquickjs::Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    match Url::parse(raw) {
        Ok(u) => decompose(&obj, &u)?,
        Err(_) => {
            obj.set("href", raw)?;
            for k in ["protocol", "host", "hostname", "port", "pathname", "search", "hash", "origin"]
            {
                obj.set(k, "")?;
            }
        }
    }
    Ok(obj)
}

/// Back the prelude's `new URL(spec, base)`. Unlike `location` (a fact that
/// empties on failure), an invalid URL is a `TypeError`: the object carries a
/// `valid` flag the prelude converts to a throw. Base resolution rides the `url`
/// crate's `Url::join`, so relative-URL semantics stay in one place (js.md §7).
fn url_parse<'js>(ctx: &Ctx<'js>, spec: &str, base: Option<&str>) -> rquickjs::Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    let parsed = match base {
        Some(b) => Url::parse(b).ok().and_then(|u| u.join(spec).ok()),
        None => Url::parse(spec).ok(),
    };
    match parsed {
        Some(u) => {
            obj.set("valid", true)?;
            decompose(&obj, &u)?;
        }
        None => obj.set("valid", false)?,
    }
    Ok(obj)
}

#[cfg(test)]
mod tests;
