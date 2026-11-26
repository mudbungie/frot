//! The page state a script sets without a mark in the tree (`bl-bc6d`):
//! focus and the top layer, written into the arena ([`crate::dom::live`]) so
//! the selector engine and `document.activeElement` read one answer. What
//! may take focus is [`crate::css::focusable`] — the rule `autofocus` obeys.

use rquickjs::function::Opt;
use rquickjs::{Ctx, Function};

use super::Host;
use crate::css::{active_in, blur_target, focus_target, focused_element, pick, picked};
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
    // Focus `id` — or the element a host delegates focus to — if it can take
    // it, or, `force`d, a dialog with nothing focusable inside, which takes
    // focus itself; answers the element focused, `undefined` when none.
    bind!(ctx, g, "__frot_focus", {
        let d = doc.clone();
        move |id: u32, force: Opt<bool>| {
            let doc = d.borrow();
            let dialog = doc.element(id).is_some_and(|el| el.name == "dialog");
            let to = focus_target(&doc, id).or((force.0 == Some(true) && dialog).then_some(id));
            drop(doc);
            if let Some(to) = to {
                d.borrow_mut().set_focus(Focus::On(to));
            }
            to
        }
    });
    // The autofocus flush (`bl-4248`), which the host runs once, after
    // `DOMContentLoaded` (`prelude/loop.js`).
    bind!(ctx, g, "__frot_autofocus", {
        let (d, geo) = (doc.clone(), h.geo.clone());
        move || geo.borrow().settle_autofocus(&mut d.borrow_mut())
    });
    // `activeElement` of the document or a shadow root.
    bind!(ctx, g, "__frot_active", {
        let d = doc.clone();
        move |root: u32| active_in(&d.borrow(), root)
    });
    // Take focus away — from `id`, only if `id.blur()` takes it ([`blur_target`]);
    // answers the element that lost it.
    bind!(ctx, g, "__frot_blur", {
        let d = doc.clone();
        move |id: Opt<u32>| {
            let doc = d.borrow();
            let was = match id.0 {
                Some(id) => blur_target(&doc, id),
                None => focused_element(&doc),
            };
            let off = id.0.is_none() || was.is_some();
            drop(doc);
            if off {
                d.borrow_mut().set_focus(Focus::Off);
            }
            was
        }
    });
    bind!(ctx, g, "__frot_popover", {
        let d = doc.clone();
        move |id: u32, open: Opt<Option<bool>>| {
            match open.0.flatten() {
                Some(true) => d.borrow_mut().show_popover(id),
                Some(false) => d.borrow_mut().set_popover(id, false),
                None => {}
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
        move |id: u32, modal: bool| match modal {
            true => d.borrow_mut().show_modal(id),
            false => d.borrow_mut().set_modal(id, false),
        }
    });
    Ok(())
}
