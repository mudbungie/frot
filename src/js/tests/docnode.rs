//! The document is an arena node (`bl-196a`, identity.md §3.21): what a page
//! can see of that before `document` itself becomes a `Node` (`bl-9f79`) — a
//! root's parent, the tree's refusals, and the queries that no longer need a
//! document to stand in.

use super::sess;

const PAGE: &str = "<!DOCTYPE html><html><body><p id=p>hi</p></body></html>";

fn eval(expr: &str) -> String {
    sess(PAGE)
        .eval(&format!(
            "(function(){{var D=document,H=D.documentElement,p=D.getElementById('p');\
             function t(f){{try{{return String(f())}}catch(e){{return e.name+': '+e.message}}}}\
             return String({expr})}})()"
        ))
        .unwrap()
}

#[test]
fn a_roots_parent_is_the_document() {
    for (expr, want) in [
        ("H.parentNode===D", "true"),
        ("H.parentElement", "null"),
        (
            "[H.previousSibling.nodeType,String(H.nextSibling)]",
            "10,null",
        ),
        (
            "[p.getRootNode()===D,H.getRootNode()===D,p.isConnected]",
            "true,true,true",
        ),
        (
            "(function(){var d=D.createElement('i');return d.getRootNode()===d&&!d.isConnected})()",
            "true",
        ),
        ("H.closest('html')===H&&p.closest('div')===null", "true"),
    ] {
        assert_eq!(eval(expr), want, "{expr}");
    }
}

/// Each refusal reads as Firefox 153.0esr's `HierarchyRequestError` does, and
/// leaves the tree as it was.
#[test]
fn the_tree_refuses_what_a_browsers_does() {
    let anc = "HierarchyRequestError: Node.appendChild: The new child is an ancestor of the parent";
    for (expr, want) in [
        ("t(function(){return p.appendChild(p)})", anc),
        ("t(function(){return p.appendChild(H)})", anc),
        ("t(function(){return p.appendChild(D)})", anc),
        (
            "t(function(){return D.createElement('b').appendChild(D)})",
            "HierarchyRequestError: Node.appendChild: May not add a Document as a child",
        ),
        (
            "t(function(){return p.firstChild.appendChild(D.createElement('b'))})",
            "HierarchyRequestError: Node.appendChild: Cannot add children to a Text",
        ),
        (
            "t(function(){return D.createComment('').insertBefore(D.createElement('b'),null)})",
            "HierarchyRequestError: Node.insertBefore: Cannot add children to a Comment",
        ),
        (
            "t(function(){return p.insertBefore(H,p.firstChild)})",
            "HierarchyRequestError: Node.insertBefore: The new child is an ancestor of the parent",
        ),
        (
            "t(function(){return p.replaceChild(H,p.firstChild)})",
            "HierarchyRequestError: Node.replaceChild: The new child is an ancestor of the parent",
        ),
        (
            "t(function(){return p.replaceChild(H.previousSibling,p.firstChild)})",
            "HierarchyRequestError: Node.replaceChild: Cannot insert a DocumentType as a child of \
          an Element",
        ),
        // Refused before anything moved.
        (
            "(t(function(){p.replaceChild(H,p.firstChild)}),p.firstChild.data+p.childNodes.length)",
            "hi1",
        ),
        ("(t(function(){p.appendChild(H)}),H.parentNode===D)", "true"),
        // A valid replacement, a fragment included, still goes through.
        (
            "(function(){var b=D.createElement('b');p.replaceChild(b,p.firstChild);var f=\
          D.createDocumentFragment();f.appendChild(D.createElement('i'));p.replaceChild(f,b);\
          return p.innerHTML})()",
            "<i></i>",
        ),
    ] {
        assert_eq!(eval(expr), want.replace("\n          ", ""), "{expr}");
    }
}

/// `matches` is asked of the element where it stands, so one in no document
/// answers too; a query inside a detached subtree finds its own descendants.
#[test]
fn a_detached_subtree_matches_and_is_queried() {
    for (expr, want) in [
        (
            "(function(){var d=D.createElement('div');d.innerHTML='<b><i></i></b>';return [\
          d.matches('div'),d.firstChild.matches('div > b'),d.querySelector('b i')!==null,\
          d.querySelector('div b')!==null,d.querySelectorAll('p').length,\
          d.firstChild.closest('div')===d]})()",
            "true,true,true,true,0,true",
        ),
        (
            "[p.matches('body > p'),p.matches('div'),t(function(){return p.matches(':bogus')})]",
            "true,false,SyntaxError: unsupported selector: :bogus",
        ),
    ] {
        assert_eq!(eval(expr), want, "{expr}");
    }
}

/// An observer on the document sees a connected node's mutations through the
/// parent chain, with the document as the target of a top-level one — and a
/// refused insertion is no mutation.
#[test]
fn the_document_is_observed_as_the_node_it_is() {
    let s = sess(PAGE);
    s.eval(
        "var out=[];var mo=new MutationObserver(function(rs){rs.forEach(function(r){\
         out.push(r.type+':'+(r.target===document?'document':r.target.nodeName))})});\
         mo.observe(document,{childList:true,subtree:true,attributes:true});\
         var p=document.getElementById('p');p.setAttribute('a','1');\
         p.appendChild(document.createElement('b'));\
         try{p.appendChild(document.documentElement)}catch(e){}\
         document.createElement('u').appendChild(document.createElement('s'));\
         var H=document.documentElement;H.remove();",
    )
    .unwrap();
    assert_eq!(
        s.eval("out.join()").unwrap(),
        "attributes:P,childList:P,childList:document"
    );
}
