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

/// `document` is a `Node` (`bl-9f79`): the rows of identity.md §3.21's first
/// table, each as read off Firefox 153.0esr.
#[test]
fn the_document_is_shaped_as_the_binarys() {
    for (expr, want) in [
        ("Object.getOwnPropertyNames(D)", "location"),
        ("Object.prototype.toString.call(D)", "[object HTMLDocument]"),
        ("[D instanceof Node,D instanceof Document,D instanceof HTMLDocument,D instanceof EventTarget,\
          Object.getPrototypeOf(HTMLDocument.prototype)===Document.prototype,\
          Object.getPrototypeOf(Document.prototype)===Node.prototype,\
          Object.getOwnPropertyNames(HTMLDocument.prototype)]",
         "true,true,true,true,true,true,constructor"),
        ("[D.nodeType,D.nodeName,D.parentNode,D.parentElement,D.nextSibling,D.nodeValue,\
          D.textContent,D.ownerDocument,D.isConnected,D.hasChildNodes()].map(String)",
         "9,#document,null,null,null,null,null,null,true,true"),
        ("Array.prototype.map.call(D.childNodes,function(n){return n.nodeName+n.nodeType})",
         "html10,HTML1"),
        ("[D.firstChild.nodeType,D.lastChild===H,D.contains(D.body),D.contains(D),\
          D.body.contains(D),D.contains(D.createElement('p')),D.getRootNode()===D]",
         "10,true,true,true,false,false,true"),
        ("(D.textContent='x',D.childNodes.length)", "2"),
        ("[D.readyState,D.compatMode,D.defaultView===globalThis,D.implementation===\
          D.implementation,D.children.length,D.documentElement===H,D.head.tagName,\
          D.getElementById('p')===p,D.getElementsByTagName('p').length,D.activeElement===D.body,\
          D.hidden,D.visibilityState,D.hasFocus(),String(D.onreadystatechange)]",
         "loading,CSS1Compat,true,true,1,true,HEAD,true,1,true,false,visible,true,null"),
        // The tree operations a page calls on it exist, and refuse as measured.
        ("(function(){var c=D.appendChild(D.createComment('z'));var n=D.childNodes.length;\
          D.removeChild(c);return n+','+D.childNodes.length})()", "3,2"),
        ("t(function(){return D.appendChild(D.createTextNode('x'))})",
         "HierarchyRequestError: Node.appendChild: Cannot insert a Text as a child of a Document"),
        ("t(function(){return D.appendChild(D.createElement('div'))})",
         "HierarchyRequestError: Node.appendChild: Cannot have more than one Element child of a \
          Document"),
        ("t(function(){return D.appendChild(H)})",
         "HierarchyRequestError: Node.appendChild: Cannot have more than one Element child of a \
          Document"),
        ("t(function(){return D.replaceChild(D.createTextNode('x'),H)})",
         "HierarchyRequestError: Node.replaceChild: Cannot insert a Text as a child of a Document"),
        ("(function(){var n=D.createElement('html');D.replaceChild(n,H);var r=D.documentElement===n;\
          D.replaceChild(H,n);return r&&D.documentElement===H})()", "true"),
        ("(function(){D.removeChild(H);var r=String(D.documentElement)+D.childNodes.length;\
          D.appendChild(H);return r+(D.body!==null)})()", "null1true"),
        ("(function(){var n=0;D.addEventListener('x',function(e){n+=e.target===D});\
          D.dispatchEvent(new Event('x'));return n+String(Object.prototype.hasOwnProperty.call(D,\
          'addEventListener'))})()", "1false"),
    ] {
        assert_eq!(eval(expr), want.replace("\n          ", ""), "{expr}");
    }
}

/// `bl-3d15`, each row read off Firefox 153.0esr: a doctype names itself and
/// carries its public and system identifiers; `document.doctype` is it, or
/// `null`; a comment's data is writable like a text node's, `null` writing ''.
#[test]
fn a_doctype_and_character_data_read_as_the_binarys() {
    let old = "<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.01//EN\" \"http://www.w3.org/TR/html4/\
               strict.dtd\"><html><body></body></html>";
    for (page, want) in [
        (PAGE, "html,html,,,true,10"),
        (
            old,
            "html,html,-//W3C//DTD HTML 4.01//EN,http://www.w3.org/TR/html4/strict.dtd,true,10",
        ),
    ] {
        let got = sess(page)
            .eval(
                "var d=document.doctype;String([d.nodeName,d.name,d.publicId,d.systemId,\
                 d===document.firstChild,d.nodeType])",
            )
            .unwrap();
        assert_eq!(got, want);
    }
    assert_eq!(
        sess("<p>x</p>").eval("String(document.doctype)").unwrap(),
        "null"
    );
    for (expr, want) in [
        (
            "(function(){var c=D.createComment('a');c.data='xyz';return [c.data,c.nodeValue,\
             c.textContent,c.length]})()",
            "xyz,xyz,xyz,3",
        ),
        (
            "(function(){var c=D.createComment('a'),x=D.createTextNode('b');c.data=null;\
             x.nodeValue=null;return JSON.stringify([c.data,x.data])})()",
            "[\"\",\"\"]",
        ),
    ] {
        assert_eq!(eval(expr), want, "{expr}");
    }
}

/// Cloning a doctype makes a doctype (bl-ad2d), its three strings copied —
/// what Firefox 153.0esr answers for this page's XHTML doctype.
#[test]
fn a_cloned_doctype_is_a_doctype() {
    let s = sess(
        "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \
         \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\"><html><body></body></html>",
    );
    let got = s
        .eval(
            "(function(){var d=document.doctype,c=d.cloneNode(),c2=d.cloneNode(true);\
             return [c.nodeType,c.nodeName,c.name,c.publicId,c.systemId,c===d,c.parentNode,\
             c.ownerDocument===document,c2.childNodes.length,\
             Object.prototype.toString.call(c),c instanceof DocumentType].join('|')})()",
        )
        .unwrap();
    assert_eq!(
        got,
        "10|html|html|-//W3C//DTD XHTML 1.0 Strict//EN|\
         http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd|false||true|0|\
         [object DocumentType]|true"
    );
}
