//! What `ElementInternals` reads of the tree and writes into the arena (js.md
//! §15.6): a form-associated element's form owner and labels, and the facts
//! the cascade and `ax` read back — form association, custom states,
//! validity and ARIA defaults ([`crate::dom::live::internals`]).

use rquickjs::{Ctx, Function};

use super::Host;

pub(super) fn install<'js>(
    ctx: &Ctx<'js>,
    g: &rquickjs::Object<'js>,
    h: &Host,
) -> rquickjs::Result<()> {
    let doc = &h.doc;
    bind!(ctx, g, "__frot_form_owner", {
        let d = doc.clone();
        move |id: u32| d.borrow().form_owner(id)
    });
    bind!(ctx, g, "__frot_labels", {
        let d = doc.clone();
        move |id: u32| d.borrow().labels(id)
    });
    bind!(ctx, g, "__frot_set_face", {
        let d = doc.clone();
        move |id: u32| d.borrow_mut().set_internals(id, |i| i.face = true)
    });
    bind!(ctx, g, "__frot_states", {
        let d = doc.clone();
        move |id: u32| {
            d.borrow()
                .internals(id)
                .map(|i| i.states.clone())
                .unwrap_or_default()
        }
    });
    bind!(ctx, g, "__frot_set_states", {
        let d = doc.clone();
        move |id: u32, states: Vec<String>| d.borrow_mut().set_internals(id, |i| i.states = states)
    });
    bind!(ctx, g, "__frot_set_invalid", {
        let d = doc.clone();
        move |id: u32, on: bool| d.borrow_mut().set_internals(id, |i| i.invalid = on)
    });
    // The ARIA default `name` (an attribute name) the internals of `id` set.
    bind!(ctx, g, "__frot_aria_default", {
        let d = doc.clone();
        move |id: u32, name: String| d.borrow().aria_default(id, &name).map(str::to_owned)
    });
    bind!(ctx, g, "__frot_set_aria_default", {
        let d = doc.clone();
        move |id: u32, name: String, value: Option<String>| {
            d.borrow_mut().set_internals(id, |i| {
                i.aria.retain(|(k, _)| *k != name);
                i.aria.extend(value.map(|v| (name, v)));
            })
        }
    });
    Ok(())
}
