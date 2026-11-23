//! `Image` and `HTMLImageElement` (`bl-1404`), against what Firefox 153.0esr
//! does with the same calls (identity.md §3.20); what an image's `src` does is
//! `imageload`'s.

use super::{sess, Session};
use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::{Env, StyleSource};

fn caught(s: &Session, expr: &str) -> String {
    s.eval(&format!(
        "(function(){{try{{return String({expr})}}catch(e){{return e.name+': '+e.message}}}})()"
    ))
    .unwrap()
}

#[test]
fn image_is_firefox_shaped() {
    let s = sess("<html><body></body></html>");
    for (expr, want) in [
        ("typeof Image", "function"),
        ("Image.length", "0"),
        ("Image.name", "Image"),
        ("String(Image)", "function Image() {\n    [native code]\n}"),
        ("Image.prototype===HTMLImageElement.prototype", "true"),
        ("HTMLImageElement.length", "0"),
        ("Object.getOwnPropertyNames(new Image()).join()", ""),
        ("Image()", "TypeError: Image constructor: 'new' is required"),
        ("new HTMLImageElement()", "TypeError: Illegal constructor."),
        (
            "['Image','HTMLImageElement'].map(function(k){var d=Object\
             .getOwnPropertyDescriptor(globalThis,k);return [typeof d.value,d.writable,\
             d.enumerable,d.configurable]})",
            "function,true,false,true,function,true,false,true",
        ),
    ] {
        assert_eq!(caught(&s, expr), want, "{expr}");
    }
}

/// `new Image()` is an `<img>` in no tree, in the page's own document; the
/// optional arguments are its `width` and `height` attributes.
#[test]
fn new_image_is_a_detached_img_element() {
    let s = sess("<html><body><img id=a src=/a.png><p id=p></p></body></html>");
    for (expr, want) in [
        (
            "(function(){var i=new Image();return [i.tagName,i.nodeName,i.parentNode,\
             i.ownerDocument===document,i.outerHTML]})()",
            "IMG,IMG,,true,<img>",
        ),
        (
            "(function(){var i=new Image();return [i instanceof Image,\
             i instanceof HTMLImageElement,i instanceof HTMLElement]})()",
            "true,true,true",
        ),
        // A parsed or created <img> is one too; nothing else is.
        (
            "[document.getElementById('a') instanceof HTMLImageElement,\
             document.createElement('img') instanceof Image,\
             document.getElementById('p') instanceof HTMLImageElement,\
             ({}) instanceof Image, null instanceof Image]",
            "true,true,false,false,false",
        ),
        (
            "(function(){var i=new Image(10,20);return [i.outerHTML,i.width,i.height]})()",
            "<img width=\"10\" height=\"20\">,10,20",
        ),
        ("new Image('7').outerHTML", "<img width=\"7\">"),
        (
            "new Image(-1, 2.7).outerHTML",
            "<img width=\"0\" height=\"2\">",
        ),
        // The height is only read when a width was given.
        ("new Image(undefined, 5).outerHTML", "<img>"),
        (
            "(function(){var i=new Image();i.width=5;i.height='9';return [i.outerHTML,i.width]})()",
            "<img width=\"5\" height=\"9\">,5",
        ),
        ("[new Image().width, new Image().height]", "0,0"),
        // It can join the tree like any element.
        (
            "(function(){var i=new Image();i.src='/b.png';document.body.appendChild(i);\
             return document.querySelectorAll('img').length})()",
            "2",
        ),
    ] {
        assert_eq!(caught(&s, expr), want, "{expr}");
    }
}

/// The members an `<img>` has live on `HTMLImageElement.prototype`, so no other
/// element has them, and the canvas keeps its own `width`/`height`.
#[test]
fn the_image_members_belong_to_img_alone() {
    let s = sess("<html><body><p id=p></p></body></html>");
    for (expr, want) in [
        (
            "(function(){var p=document.getElementById('p');return [p.complete,p.naturalWidth,\
             p.naturalHeight,p.currentSrc,p.width,p.height].map(String)})()",
            "undefined,undefined,undefined,undefined,undefined,undefined",
        ),
        (
            "(function(){var c=document.createElement('canvas');c.width=7;\
             return [c.width,c.height,c.complete]})()",
            "7,150,",
        ),
        (
            "(function(){var p=document.getElementById('p');p.width=4;return p.outerHTML})()",
            "<p id=\"p\"></p>",
        ),
        (
            "['complete','naturalWidth','width'].map(function(k){var d=Object\
             .getOwnPropertyDescriptor(HTMLImageElement.prototype,k);return d.get.name+'|'+String(d.get)\
             .indexOf('native code')})",
            "get complete|27,get naturalWidth|31,get width|24",
        ),
    ] {
        assert_eq!(caught(&s, expr), want, "{expr}");
    }
}

/// No request leaves frot for an image: assigning `src` reflects it, and a
/// beacon (`new Image().src = url`) is not sent. The image fails instead, the
/// way it does in a Firefox with images blocked (bl-91a4; imageload.rs has the
/// rest of the rule).
#[test]
fn an_image_is_never_requested() {
    let mut server = mockito::Server::new();
    let beacon = server.mock("GET", mockito::Matcher::Any).expect(0).create();
    let env = Env {
        url: format!("{}/page", server.url()),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
    };
    let s = Session::new(
        Document::parse("<html><body><img id=a src=/a.png></body></html>"),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
    );
    s.eval("var i=new Image(); i.src='/pixel.gif?id=1'; document.body.appendChild(i)")
        .unwrap();
    let src = format!("{}/pixel.gif?id=1", server.url());
    let read =
        "[i.src,i.getAttribute('src'),i.complete,i.naturalWidth,i.naturalHeight,i.currentSrc]";
    assert_eq!(
        s.eval(read).unwrap(),
        format!("{src},/pixel.gif?id=1,false,0,0,{src}")
    );
    // The parsed <img> and the beacon both end, failed, and nothing was fetched.
    s.eval("while(__frot_next_image()>=0);").unwrap();
    beacon.assert();
    assert_eq!(
        s.eval(read).unwrap(),
        format!("{src},/pixel.gif?id=1,true,0,0,{src}")
    );
    assert_eq!(
        s.eval("document.getElementById('a').complete").unwrap(),
        "true"
    );
}
