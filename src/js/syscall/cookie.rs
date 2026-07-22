//! `document.cookie` syscalls (bl-6dad, js.md §7) — the JS view onto the one
//! shared [`crate::fetch::CookieJar`] the transport owns. No second jar: the
//! prelude's `document.cookie` get/set delegate straight here, so `Set-Cookie`
//! from the document GET and JS writes live in one place.
//!
//! `__frot_cookie_get()` returns the applicable, **non-HttpOnly** cookies at the
//! final document URL (`host.env.url`) — HttpOnly never enters JS. `__frot_cookie_set(v)`
//! parses a `document.cookie` write into the same jar (browser-allowed attributes
//! only; a JS write can never mint an HttpOnly cookie). Later eligible GETs
//! observe the write because they read the same jar.

use rquickjs::{Ctx, Function, Object};

use super::Host;

/// Bind one named host function, keeping the `?` paths on the call line so line
/// coverage sees the executed `set` (the core table's `bind!` shape).
macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

/// Register `__frot_cookie_get` / `__frot_cookie_set`, capturing a clone of the
/// shared jar and the final document URL.
pub fn install<'js>(ctx: &Ctx<'js>, g: &Object<'js>, host: &Host) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_cookie_get", {
        let (jar, url) = (host.cookie.clone(), host.env.url.clone());
        move || jar.lock().unwrap().document_cookie(&url)
    });
    bind!(ctx, g, "__frot_cookie_set", {
        let (jar, url) = (host.cookie.clone(), host.env.url.clone());
        move |value: String| jar.lock().unwrap().write_script(&value, &url)
    });
    Ok(())
}
