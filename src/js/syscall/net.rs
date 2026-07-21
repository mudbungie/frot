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

use rquickjs::{Array, Ctx, Function, Object, String as JsString, Value};

use super::Host;
use crate::js::subfetch::Outcome;

/// Bind one named host function, keeping the `?` paths on the call line so line
/// coverage sees the executed `set` (the core table's `bind!` shape).
macro_rules! bind {
    ($ctx:expr, $g:expr, $name:literal, $f:expr) => {
        $g.set($name, Function::new($ctx.clone(), $f)?.with_name($name)?)?
    };
}

/// Register `__frot_subfetch` on the realm, capturing a clone of the shared
/// once-then-frozen cache (§6).
pub fn install<'js>(ctx: &Ctx<'js>, g: &Object<'js>, host: &Host) -> rquickjs::Result<()> {
    bind!(ctx, g, "__frot_subfetch", {
        let sf = host.subfetch.clone();
        move |ctx: Ctx<'js>, url: String| -> rquickjs::Result<Object<'js>> {
            let outcome = sf.borrow_mut().get(&url, crate::fetch::Intent::FetchXhr);
            outcome_obj(&ctx, outcome)
        }
    });
    Ok(())
}

/// Shape an [`Outcome`] into the prelude's result object. A failure sets only
/// `error`; a success sets the response fields and a `[[name, value], …]`
/// headers array.
///
/// The success fields are built first and written in one uniform pass. That
/// keeps the two kinds of step honest and separate: converting a Rust value
/// into a JS one either allocates (a string, the headers array) and is spelled
/// fallibly, or cannot fail at all (a bool, an integer) and is spelled
/// infallibly — rather than every field riding a `?` that, for the immediates,
/// can never fire. Writing the fields *is* uniformly fallible, so it is one
/// loop with one failure path instead of five interleaved ones.
fn outcome_obj<'js>(ctx: &Ctx<'js>, outcome: Outcome) -> rquickjs::Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    match outcome {
        Outcome::Failed(reason) => obj.set("error", reason)?,
        Outcome::Got(f) => {
            let headers = Array::new(ctx.clone())?;
            for (i, (name, value)) in f.headers.into_iter().enumerate() {
                let pair = Array::new(ctx.clone())?;
                pair.set(0, name)?;
                pair.set(1, value)?;
                headers.set(i, pair)?;
            }
            let fields = [
                ("ok", Value::new_bool(ctx.clone(), f.ok)),
                ("status", Value::new_int(ctx.clone(), f.status.into())),
                ("url", js_string(ctx, &f.url)?),
                ("body", js_string(ctx, &f.body)?),
                ("headers", headers.into_value()),
            ];
            for (name, value) in fields {
                obj.set(name, value)?;
            }
        }
    }
    Ok(obj)
}

/// A JS string value. Allocating, hence fallible — unlike the immediates above.
fn js_string<'js>(ctx: &Ctx<'js>, s: &str) -> rquickjs::Result<Value<'js>> {
    Ok(JsString::from_str(ctx.clone(), s)?.into_value())
}

#[cfg(test)]
mod tests;
