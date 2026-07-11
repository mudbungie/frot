//! The subfetch syscall (js.md §6) — the single host op behind `fetch`,
//! `XMLHttpRequest`, and external `<script src>`. GET-only by construction (it
//! only ever drives [`crate::js::subfetch`], which GETs); method policy is the
//! prelude's, since a non-GET never reaches the wire.
//!
//! `__frot_subfetch(url)` returns a plain object the prelude shapes into a
//! `Response`/XHR: a success carries `ok`/`status`/`url`/`body`/`headers`; a
//! refusal or transport failure carries only `error` (a string), which the
//! prelude turns into a rejected promise / XHR error — counted through the §10
//! unified failure channel like any other unhandled error.

use rquickjs::{Array, Ctx, Function, Object};

use crate::js::subfetch::{Outcome, SharedSubfetch};

/// Bind one named host function, keeping the `?` paths on the call line so line
/// coverage sees the executed `set` (the core table's `bind!` shape).
macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

/// Register `__frot_subfetch` on the realm, capturing a clone of the shared
/// once-then-frozen cache (§6).
pub fn install<'js>(
    ctx: &Ctx<'js>,
    g: &Object<'js>,
    subfetch: &SharedSubfetch,
) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_subfetch", {
        let sf = subfetch.clone();
        move |ctx: Ctx<'js>, url: String| -> rquickjs::Result<Object<'js>> {
            let outcome = sf.borrow_mut().get(&url);
            outcome_obj(&ctx, outcome)
        }
    });
    Ok(())
}

/// Shape an [`Outcome`] into the prelude's result object. A failure sets only
/// `error`; a success sets the response fields and a `[[name, value], …]`
/// headers array.
fn outcome_obj<'js>(ctx: &Ctx<'js>, outcome: Outcome) -> rquickjs::Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    match outcome {
        Outcome::Failed(reason) => obj.set("error", reason)?,
        Outcome::Got(f) => {
            obj.set("ok", f.ok)?;
            obj.set("status", f.status)?;
            obj.set("url", f.url)?;
            obj.set("body", f.body)?;
            let headers = Array::new(ctx.clone())?;
            for (i, (name, value)) in f.headers.into_iter().enumerate() {
                let pair = Array::new(ctx.clone())?;
                pair.set(0, name)?;
                pair.set(1, value)?;
                headers.set(i, pair)?;
            }
            obj.set("headers", headers)?;
        }
    }
    Ok(obj)
}
