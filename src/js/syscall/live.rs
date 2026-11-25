//! The page state a script sets without a mark in the tree (`bl-bc6d`):
//! focus and the top layer, written into the arena ([`crate::dom::live`]) so
//! the selector engine and `document.activeElement` read one answer. What
//! may take focus is [`crate::css::focusable`] — the rule `autofocus` obeys.

use rquickjs::function::Opt;
use rquickjs::{Ctx, Function};

use super::Host;
use crate::css::{focusable, focused_element, pick, picked};
use crate::dom::live::Focus;

pub(super) fn install<'js>(
    ctx: &Ctx<'js>,
    g: &rquickjs::Object<'js>,
    h: &Host,
) -> rquickjs::Result<()> {
    let doc = &h.doc;
    bind!(ctx, g, "__frot_focused", {
        let d = doc.clone();
        move || focused_element(&d.borrow())
    });
    // Focus `id` if it can take focus — or, `force`d, a dialog with nothing
    // focusable inside, which takes focus itself; answers whether it took it.
    bind!(ctx, g, "__frot_focus", {
        let d = doc.clone();
        move |id: u32, force: Opt<bool>| {
            let doc = d.borrow();
            let can = doc.element(id).is_some_and(|el| {
                focusable(&doc, id, el) || (force.0 == Some(true) && el.name == "dialog")
            });
            drop(doc);
            if can {
                d.borrow_mut().set_focus(Focus::On(id));
            }
            can
        }
    });
    bind!(ctx, g, "__frot_blur", {
        let d = doc.clone();
        move || d.borrow_mut().set_focus(Focus::Off)
    });
    bind!(ctx, g, "__frot_popover", {
        let d = doc.clone();
        move |id: u32, open: Opt<Option<bool>>| {
            if let Some(open) = open.0.flatten() {
                d.borrow_mut().set_popover(id, open);
            }
            d.borrow().popover_open(id)
        }
    });
    // `checked` / `selected` (`bl-ff74`): read, or set and read back.
    bind!(ctx, g, "__frot_picked", {
        let d = doc.clone();
        move |id: u32, on: Opt<bool>| {
            if let Some(on) = on.0 {
                pick(&mut d.borrow_mut(), id, on);
            }
            let doc = d.borrow();
            doc.element(id).is_some_and(|el| picked(&doc, id, el))
        }
    });
    bind!(ctx, g, "__frot_indeterminate", {
        let d = doc.clone();
        move |id: u32, on: Opt<bool>| {
            if let Some(on) = on.0 {
                d.borrow_mut().set_indeterminate(id, on);
            }
            d.borrow().indeterminate(id)
        }
    });
    bind!(ctx, g, "__frot_modal", {
        let d = doc.clone();
        move |id: u32, modal: bool| d.borrow_mut().set_modal(id, modal)
    });
    Ok(())
}
