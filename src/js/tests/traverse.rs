//! Element traversal (`bl-59f7`): the ParentNode trio on Element, Document and
//! DocumentFragment and the NonDocumentTypeChildNode pair on Element and
//! CharacterData, against Firefox 153.0esr's answers to the same probe (the
//! identity.md §3.19 rig, 2026-11-24).

use super::sess;

const PAGE: &str = "<!DOCTYPE html><html><body><div id=d>a<i>1</i><!--c-->b<b>2</b>t<u>3</u></div>\
                    </body></html>";

const PROBE: &str = "(function(){var d=document.getElementById('d'),k=d.childNodes,\
    f=document.createDocumentFragment();f.append('x',document.createElement('em'),'y');\
    var lone=document.createElement('s');\
    function n(x){return x===null?'null':x===undefined?'undef':x.nodeName}\
    return [n(k[0].nextElementSibling),n(k[0].previousElementSibling),n(k[1].nextElementSibling),\
    n(k[2].nextElementSibling),n(k[2].previousElementSibling),n(k[3].nextElementSibling),\
    n(k[6].previousElementSibling),n(k[6].nextElementSibling),n(k[5].nextElementSibling),\
    n(document.doctype.nextElementSibling),n(lone.nextElementSibling),\
    n(f.firstChild.nextElementSibling),n(f.lastChild.previousElementSibling),\
    n(document.firstElementChild),document.childElementCount,document.children.length,\
    f.childElementCount,n(f.firstElementChild),n(f.lastElementChild),n(d.firstElementChild),\
    n(d.lastElementChild),d.childElementCount,d.children.length,\
    n(document.documentElement.nextElementSibling),'previousElementSibling' in document,\
    'nextElementSibling' in f,'nextElementSibling' in document.doctype,\
    typeof Object.getOwnPropertyDescriptor(CharacterData.prototype,'nextElementSibling').get\
    ].join()})()";

/// Firefox 153.0esr's answer, verbatim.
const FIREFOX: &str = "I,null,B,B,I,B,B,null,U,undef,null,EM,EM,HTML,1,1,1,EM,EM,I,U,3,3,null,\
                       false,false,false,function";

#[test]
fn element_traversal_answers_as_firefox_does() {
    assert_eq!(sess(PAGE).eval(PROBE).unwrap(), FIREFOX);
}
