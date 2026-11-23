//! The DOM interface chain (`bl-78cb`, identity.md §3.21) against what Firefox
//! 153.0esr reports: `tests/fixtures/js/dom-interfaces.json` holds each
//! interface's parent and its prototype's own members, in order, read off the
//! binary. frot implements a subset, so the rule is: every member frot defines
//! on an interface is one the binary defines there, with the same descriptor
//! shape, and frot's members appear in the binary's order.

use serde_json::Value;

use super::{sess, Session};

const MEASURED: &str = include_str!("../../../tests/fixtures/js/dom-interfaces.json");

/// frot's view of one interface, in the fixture's own encoding.
const DESCRIBE: &str = "(function(n){var C=globalThis[n];if(typeof C!=='function')return 'absent';\
    var P=C.prototype,pp=Object.getPrototypeOf(P);\
    function code(d){return (d.get?'g':'')+(d.set?'s':'')+(('value' in d)?(typeof d.value===\
    'function'?'f':'v'):'')+(d.enumerable?'e':'')+(d.configurable?'c':'')+(d.writable?'w':'')}\
    return JSON.stringify({parent:pp===Object.prototype?null:pp.constructor.name,\
    ctorParent:Object.getPrototypeOf(C).name||null,tag:Object.prototype.toString.call(P),\
    members:Object.getOwnPropertyNames(P).map(function(k){return k+':'+\
    code(Object.getOwnPropertyDescriptor(P,k))}).join(' ')})})";

fn measured() -> Value {
    serde_json::from_str(MEASURED).unwrap()
}

fn page() -> Session {
    sess("<!DOCTYPE html><html><body><p id=p>hi<!--c--></p></body></html>")
}

#[test]
fn every_interface_is_a_subset_of_the_binarys_in_its_order() {
    let s = page();
    let measured = measured();
    let mut built = 0;
    for (name, want) in measured["interfaces"].as_object().unwrap() {
        let got = s.eval(&format!("{DESCRIBE}('{name}')")).unwrap();
        assert_ne!(got, "absent", "{name} is not built");
        let got: Value = serde_json::from_str(&got).unwrap();
        assert_eq!(got["parent"], want["parent"], "{name}: parent");
        assert_eq!(
            got["ctorParent"], want["ctorParent"],
            "{name}: interface object's parent"
        );
        assert_eq!(
            got["tag"],
            format!("[object {name}]"),
            "{name}: toStringTag"
        );
        // Subsequence: each of frot's members, shape included, must be found
        // in the binary's list after the one before it.
        let mut rest = want["members"].as_str().unwrap().split(' ');
        for member in got["members"].as_str().unwrap().split(' ') {
            assert!(
                rest.any(|m| m == member),
                "{name}: `{member}` is not in the binary's list at this position"
            );
        }
        built += 1;
    }
    assert_eq!(built, 82);
    // And frot builds no HTML interface the binary lacks.
    let extra = s
        .eval(
            "Object.getOwnPropertyNames(globalThis).filter(function(n){return \
             /^HTML.*Element$/.test(n)}).length",
        )
        .unwrap();
    assert_eq!(extra, "71");
}

#[test]
fn an_element_implements_the_interface_the_binary_gives_its_tag() {
    let s = page();
    for (tag, want) in measured()["tags"].as_object().unwrap() {
        // The arena keeps no namespace, so a created `svg` is SVGElement where
        // the binary's HTML-namespace one is HTMLUnknownElement (identity.md §11).
        let want = if tag == "svg" {
            "SVGElement"
        } else {
            want.as_str().unwrap()
        };
        let got = s
            .eval(&format!(
                "Object.getPrototypeOf(document.createElement('{tag}')).constructor.name"
            ))
            .unwrap();
        assert_eq!(got, want, "<{tag}>");
    }
}

/// What a page sees of the chain, each value as measured on the binary.
#[test]
fn nodes_behave_as_their_interfaces() {
    let s = page();
    for (expr, want) in [
        // One wrapper per node: identity holds and a set property stays.
        ("document.body===document.body", "true"),
        ("(document.body.x=1,document.body.x)", "1"),
        ("p.firstChild===p.childNodes[0]&&p.firstChild.parentNode===p", "true"),
        // A text node has no element member; an element has no `value`.
        ("typeof t.getAttribute+typeof t.querySelector+typeof t.children", "undefinedundefinedundefined"),
        ("typeof p.value+typeof p.getContext+typeof p.href", "undefinedundefinedundefined"),
        ("[t,c,p,document.createElement('x-y')].map(String).join()",
         "[object Text],[object Comment],[object HTMLParagraphElement],[object HTMLElement]"),
        ("[p instanceof HTMLElement,p instanceof Element,p instanceof Node,p instanceof EventTarget,\
          p instanceof Text,t instanceof CharacterData,t instanceof Element,c instanceof Comment,\
          c instanceof Text,document.doctype===undefined].join()",
         "true,true,true,true,false,true,false,true,false,true"),
        ("[t.nodeType,t.nodeName,t.data,t.length,t.nodeValue,c.nodeType,c.nodeName,c.data,\
          String(p.nodeValue)].join()", "3,#text,hi,2,hi,8,#comment,c,null"),
        ("[p.hasChildNodes(),t.hasChildNodes(),p.isSameNode(p),p.isSameNode(null),p.isConnected,\
          document.createElement('p').isConnected,t.parentElement===p,\
          String(document.documentElement.parentElement),p.contains(t),t.contains(p),\
          p.contains(null)].join()",
         "true,false,true,false,true,false,true,null,true,false,false"),
        ("[Node.ELEMENT_NODE,p.TEXT_NODE,Node.NOTATION_NODE,Node.DOCUMENT_POSITION_DISCONNECTED,\
          Node.DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC].join()", "1,3,12,1,32"),
        ("JSON.stringify(Object.getOwnPropertyDescriptor(Node,'TEXT_NODE'))",
         "{\"value\":3,\"writable\":false,\"enumerable\":true,\"configurable\":false}"),
        // Text and Comment construct; nothing else in the chain does.
        ("(function(){var x=new Text('a');return x.data+x.nodeType+(x instanceof Text)+\
          (x.ownerDocument===document)+JSON.stringify(new Text().data)+new Comment('z').nodeValue})()",
         "a3truetrue\"\"z"),
        ("['Node','Element','HTMLElement','HTMLDivElement','CharacterData','DocumentType'].map(\
          function(n){try{new globalThis[n]()}catch(e){return e.message}}).join()",
         "Illegal constructor.,Illegal constructor.,Illegal constructor.,Illegal constructor.,\
          Illegal constructor.,Illegal constructor."),
        // Clones keep their kind; textContent's null and '' leave no child.
        ("(function(){var k=c.cloneNode();return k.nodeType+k.data+t.cloneNode().data+\
          p.cloneNode(true).childNodes.length+p.cloneNode().childNodes.length})()", "8chi20"),
        ("(function(){var d=document.createElement('div');d.textContent='a';var n=d.childNodes.length;\
          d.textContent=null;n+=','+d.childNodes.length;d.textContent='';return n+','+\
          d.childNodes.length+','+(t.textContent='yo',t.data)+','+(t.nodeValue='hi',p.textContent)})()",
         "1,0,0,yo,hi"),
        ("(function(){var d=document.createElement('div');d.append('a',document.createElement('b'));\
          d.firstChild.remove();d.prepend('z');return d.innerHTML+d.lastChild.previousSibling.data+\
          String(d.lastChild.nextSibling)+p.matches('body > p')+p.matches('div')})()",
         "z<b></b>znulltruefalse"),
        ("['fieldset','output','textarea','select'].map(function(n){var e=document.createElement(n);\
          e.type='x';return e.type}).join()+(function(){var e=document.createElement('select');\
          e.setAttribute('multiple','');return e.type})()",
         "fieldset,output,textarea,select-oneselect-multiple"),
        // Listeners ride the inherited EventTarget, on the node's one wrapper.
        ("(function(){var n=0;p.addEventListener('x',function(e){n+=e.target===p});\
          document.getElementById('p').dispatchEvent(new Event('x'));return n+\
          String(Object.prototype.hasOwnProperty.call(Node.prototype,'addEventListener'))})()",
         "1false"),
    ] {
        let got = s
            .eval(&format!(
                "(function(){{var p=document.getElementById('p'),t=p.firstChild,c=p.lastChild;\
                 return String({expr})}})()"
            ))
            .unwrap();
        assert_eq!(got, want.replace("\n", "").replace("          ", ""), "{expr}");
    }
}
