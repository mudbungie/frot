// isEqualNode probe (bl-d5dc): run on Firefox 153.0esr (identity.md §3.19 rig) as
// a page script over equal/page.html; its answers kept verbatim in
// firefox153.json.
function t(f){try{var v=f();return v===undefined?'undef':v===null?'null':String(v)}catch(e){return 'THROW '+e.name+':'+e.message}}
var D=document, k=D.body.children, a=k[0], b=k[1], c=k[2];
resolve({
 attrOrder: t(()=>a.isEqualNode(b)), attrMissing: t(()=>a.isEqualNode(c)), self: t(()=>a.isEqualNode(a)), clone: t(()=>a.isEqualNode(a.cloneNode(true))), shallow: t(()=>a.isEqualNode(a.cloneNode(false))),
 nul: t(()=>a.isEqualNode(null)), none: t(()=>a.isEqualNode()), obj: t(()=>a.isEqualNode({})), str: t(()=>a.isEqualNode('x')),
 text: t(()=>D.createTextNode('q').isEqualNode(D.createTextNode('q'))+','+D.createTextNode('q').isEqualNode(D.createTextNode('r'))+','+D.createTextNode('q').isEqualNode(D.createComment('q'))),
 comment: t(()=>D.createComment('q').isEqualNode(D.createComment('q'))),
 doctype: t(()=>D.doctype.cloneNode().isEqualNode(D.doctype)+','+D.implementation.createDocumentType('html','a','').isEqualNode(D.doctype)),
 frag: t(()=>{var f=D.createDocumentFragment(),g=D.createDocumentFragment();f.append('x',D.createElement('i'));g.append('x',D.createElement('i'));return f.isEqualNode(g)}),
 doc: t(()=>D.isEqualNode(D)),
 tagcase: t(()=>D.createElement('DIV').isEqualNode(D.createElement('div'))),
 svg: t(()=>D.createElementNS('http://www.w3.org/2000/svg','g').isEqualNode(D.createElement('g'))),
 len: t(()=>Node.prototype.isEqualNode.length+','+Object.keys(Object.getOwnPropertyDescriptor(Node.prototype,'isEqualNode')).join('/')),
 attrNode: t(()=>{var x=D.createAttribute('q');x.value='1';var y=D.createAttribute('q');y.value='1';return x.isEqualNode(y)}),
 doctypeClone: t(()=>D.doctype.cloneNode().isEqualNode(D.doctype)+','+D.doctype.isEqualNode(D.body)),
 attrHandle: t(()=>a.attributes[0].isEqualNode(c.attributes[0])+','+a.attributes[0].isEqualNode(a.attributes[1])),
 textSplit: t(()=>{var p=D.createElement('p'),q=D.createElement('p');p.append('ab');q.append('a','b');return p.isEqualNode(q)}),
});
