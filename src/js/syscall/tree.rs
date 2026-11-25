//! The trees beside the node tree (js.md §15): the fragment an element owns
//! outside its child list — a `<template>`'s contents — and what the custom
//! element reactions ask of the tree.

use rquickjs::{Ctx, Function};

use super::Host;

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
    Ok(())
}
