//! The trees beside the node tree (js.md §15): the fragment an element owns
//! outside its child list — a `<template>`'s contents or a shadow root —,
//! slot assignment, what the custom element reactions ask of the tree, and a
//! bare copy of a node, for the doctype `cloneNode` has no other way to make.

use rquickjs::{Ctx, Function};

use super::Host;
use crate::dom::shadow::Shadow;

pub(super) fn install<'js>(
    ctx: &Ctx<'js>,
    g: &rquickjs::Object<'js>,
    h: &Host,
) -> rquickjs::Result<()> {
    let doc = &h.doc;
    bind!(ctx, g, "__frot_content", {
        let d = doc.clone();
        move |id: u32| d.borrow().node(id).content
    });
    bind!(ctx, g, "__frot_shadow", {
        let d = doc.clone();
        move |id: u32| d.borrow().shadow_root(id)
    });
    // `[host, closed, declarative, clonable, serializable, delegatesFocus,
    // manual]` of a shadow root, `null` for any other node.
    bind!(ctx, g, "__frot_shadow_info", {
        let d = doc.clone();
        move |id: u32| {
            d.borrow().shadow(id).map(|s| {
                let flags = [s.closed, s.declarative, s.clonable, s.serializable];
                let more = [s.delegates_focus, s.manual];
                let mut v = vec![s.host];
                v.extend(flags.iter().chain(&more).map(|&f| u32::from(f)));
                v
            })
        }
    });
    // Answers why a shadow root cannot be attached, `undefined` when it was.
    bind!(ctx, g, "__frot_attach_shadow", {
        let d = doc.clone();
        move |host: u32,
              closed: bool,
              clonable: bool,
              serializable: bool,
              focus: bool,
              manual: bool| {
            let init = Shadow {
                host,
                closed,
                declarative: false,
                clonable,
                serializable,
                delegates_focus: focus,
                manual,
            };
            d.borrow_mut().attach_shadow(init).err()
        }
    });
    bind!(ctx, g, "__frot_tree_root", {
        let d = doc.clone();
        move |id: u32| d.borrow().tree_root(id)
    });
    bind!(ctx, g, "__frot_assigned", {
        let d = doc.clone();
        move |id: u32| d.borrow().assigned(id)
    });
    bind!(ctx, g, "__frot_assigned_slot", {
        let d = doc.clone();
        move |id: u32| d.borrow().assigned_slot(id)
    });
    // The text of each sheet `root` (the document or a shadow root) adopted.
    bind!(ctx, g, "__frot_adopt", {
        let d = doc.clone();
        move |root: u32, sheets: Vec<String>| d.borrow_mut().set_adopted(root, sheets)
    });
    bind!(ctx, g, "__frot_connected", {
        let d = doc.clone();
        move |id: u32| d.borrow().connected(id)
    });
    bind!(ctx, g, "__frot_custom_nodes", {
        let d = doc.clone();
        move |id: u32| d.borrow().custom_nodes(id)
    });
    bind!(ctx, g, "__frot_set_custom", {
        let d = doc.clone();
        move |id: u32| d.borrow_mut().set_custom(id)
    });
    bind!(ctx, g, "__frot_copy_node", {
        let d = doc.clone();
        move |id: u32| d.borrow_mut().copy_node(id)
    });
    Ok(())
}
