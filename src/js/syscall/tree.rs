//! The trees beside the node tree (js.md §15): the fragment an element owns
//! outside its child list — a `<template>`'s contents.

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
    Ok(())
}
