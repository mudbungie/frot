//! `DocumentFragment` as the arena node it is (`bl-de06`), each value as read
//! off Firefox 153.0esr (identity.md §3.21): a parentless holder that is a
//! `Node`, whose children the tree moves when it is inserted.

use super::sess;

fn eval(expr: &str) -> String {
    sess("<!DOCTYPE html><html><body><p id=p>hi</p></body></html>")
        .eval(&format!(
            "(function(){{var D=document;\
             function t(f){{try{{return String(f())}}catch(e){{return e.name+': '+e.message}}}}\
             function E(n){{return D.createElement(n||'b')}}\
             function F(){{var f=D.createDocumentFragment();for(var i=0;i<arguments.length;i++)\
             f.appendChild(arguments[i]);return f}}\
             return String({expr})}})()"
        ))
        .unwrap()
}

#[test]
fn a_fragment_is_a_node_that_holds_children() {
    for (expr, want) in [
        ("(function(){var f=new DocumentFragment();return [f.nodeType,f.nodeName,f.parentNode===null\
          ?'':1,f.nodeValue===null?'':1,f.textContent==='',f.ownerDocument===D,f.isConnected,String(f),\
          f instanceof Node,Object.getOwnPropertyNames(f).length].join('|')})()",
         "11|#document-fragment|||true|true|false|[object DocumentFragment]|true|0"),
        ("(function(){var k=E('i'),f=F(k);return [k.parentNode===f,k.parentElement,\
          k.getRootNode()===f,k.isConnected,f.firstChild===k,f.lastChild===k,f.hasChildNodes(),\
          f.contains(k)].join('|')})()", "true||true|false|true|true|true|true"),
        ("(function(){var k=E('i'),f=F(k);f.removeChild(k);return f.childNodes.length+'|'+\
          k.parentNode})()", "0|null"),
        ("(function(){var f=F(E('i'),D.createTextNode('x'));f.firstChild.textContent='q';var a=\
          f.textContent;f.textContent='zz';return a+'|'+f.childNodes.length+'|'+\
          f.firstChild.nodeType})()", "qx|1|3"),
        ("(function(){var f=F(E('i'));f.firstChild.textContent='x';var s=f.cloneNode(false),\
          d=f.cloneNode(true);return [s.nodeType,s.childNodes.length,d.childNodes.length,\
          d.firstChild!==f.firstChild,d.textContent].join('|')})()", "11|0|1|true|x"),
        ("(function(){var f=F(),n=0;f.addEventListener('x',function(e){n+=(e.target===f)});\
          f.dispatchEvent(new Event('x'));return n})()", "1"),
        // ParentNode, with a getElementById an element has not got.
        ("(function(){var f=F(E('i'),E('b'));f.firstChild.id='q';f.firstChild.appendChild(E('u'));\
          return [f.querySelector('i u')!==null,f.querySelectorAll('*').length,\
          f.getElementById('q')===f.firstChild,f.getElementById('zz'),f.children.length,\
          f.querySelector('b')===f.lastChild,f.firstChild.matches('i'),\
          typeof E().getElementById].join('|')})()", "true|3|true||2|true|true|undefined"),
        ("(function(){var f=F(E('i'));f.append('t',E('b'));f.prepend('s');return Array.prototype\
          .map.call(f.childNodes,function(x){return x.nodeName}).join()})()", "#text,I,#text,B"),
        ("Element.prototype.append!==DocumentFragment.prototype.append", "true"),
    ] {
        assert_eq!(eval(expr), want, "{expr}");
    }
}

#[test]
fn inserting_a_fragment_moves_its_children() {
    for (expr, want) in [
        ("(function(){var f=F(E('i'),D.createTextNode('x')),k=f.firstChild,d=E('div');var r=\
          d.appendChild(f);return [r===f,f.childNodes.length,d.innerHTML,k.parentNode===d,\
          f.parentNode===null].join('|')})()", "true|0|<i></i>x|true|true"),
        ("(function(){var d=E('div');d.appendChild(F());return d.childNodes.length})()", "0"),
        ("(function(){var d=E('div');d.innerHTML='<u></u>';d.insertBefore(F(E('i'),E('b')),\
          d.firstChild);return d.innerHTML})()", "<i></i><b></b><u></u>"),
        ("(function(){var d=E('div');d.innerHTML='<u></u><s></s>';d.replaceChild(F(E('i'),E('b')),\
          d.firstChild);return d.innerHTML})()", "<i></i><b></b><s></s>"),
        ("(function(){var a=F(E('i')),b=F(E('u'));b.appendChild(a);return b.childNodes.length+'|'+\
          a.childNodes.length})()", "2|0"),
        // innerHTML and outerHTML ride the same move.
        ("(function(){var p=D.getElementById('p');p.innerHTML='<i>a</i>b<u></u>';\
          p.firstChild.outerHTML='<s></s>x';return p.innerHTML+'|'+p.childNodes.length})()",
         "<s></s>xb<u></u>|4"),
    ] {
        assert_eq!(eval(expr), want, "{expr}");
    }
}

#[test]
fn a_fragment_is_refused_for_what_it_holds() {
    let h = "HierarchyRequestError: Node.appendChild: ";
    for (expr, want) in [
        (
            "t(function(){var a=F(E('i'));return a.appendChild(a)})",
            format!("{h}The new child is an ancestor of the parent"),
        ),
        (
            "t(function(){var k=E('i'),a=F(k);return k.appendChild(a)})",
            format!("{h}The new child is an ancestor of the parent"),
        ),
        (
            "t(function(){return F().appendChild(D.documentElement.previousSibling)})",
            format!("{h}Cannot insert a DocumentType as a child of a DocumentFragment"),
        ),
        (
            "t(function(){return F().appendChild(D)})",
            format!("{h}May not add a Document as a child"),
        ),
    ] {
        assert_eq!(eval(expr), want, "{expr}");
    }
}

/// One record for the run a fragment moves, with the run's neighbours, and the
/// removal seen by an observer of the fragment.
#[test]
fn a_fragment_move_is_one_mutation_record() {
    let s = sess("<html><body><p id=p><u></u></p></body></html>");
    s.eval(
        "var out=[];function log(tag){return function(rs){rs.forEach(function(r){out.push(tag+':'+\
         r.addedNodes.length+'+'+r.removedNodes.length+':'+\
         (r.previousSibling&&r.previousSibling.nodeName)+':'+(r.nextSibling&&r.nextSibling.nodeName))})}}\
         var p=document.getElementById('p'),f=document.createDocumentFragment();\
         f.appendChild(document.createElement('i'));f.appendChild(document.createElement('b'));\
         new MutationObserver(log('p')).observe(p,{childList:true});\
         new MutationObserver(log('f')).observe(f,{childList:true});\
         p.appendChild(f);p.appendChild(f);p.insertBefore(document.createDocumentFragment(),null);",
    )
    .unwrap();
    assert_eq!(
        s.eval("out.sort().join()").unwrap(),
        "f:0+2:null:null,p:2+0:U:null"
    );
}
