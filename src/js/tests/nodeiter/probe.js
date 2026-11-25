// NodeIterator probe (bl-33df): run on Firefox 153.0esr (identity.md §3.19 rig)
// as a page script over nodeiter/page.html; its answers kept verbatim in
// firefox153.json.
function t(f){try{var v=f();return v===undefined?'undef':v===null?'null':String(v)}catch(e){return 'THROW '+e.name+':'+e.message}}
var D=document, R=D.getElementById('r');
function nm(n){return n===null?'null':n.nodeType===1?n.id||n.localName:n.nodeType===3?'#'+n.data:n.nodeType===8?'!'+n.data:n.nodeName}
function all(it,back){var o=[],n;while((n=back?it.previousNode():it.nextNode()))o.push(nm(n));return o.join(' ')}
function st(it){return nm(it.referenceNode)+'/'+it.pointerBeforeReferenceNode}
function fresh(){R.innerHTML='<p id=a>A<i id=ai>i</i></p><!--c--><p id=b>B</p><p id=c>C</p>';}
resolve({
 proto: t(()=>Object.getOwnPropertyNames(NodeIterator.prototype).join(' ')),
 ctor: t(()=>new NodeIterator()),
 len: t(()=>D.createNodeIterator.length),
 fresh: t(()=>{var it=D.createNodeIterator(R);return [nm(it.root),st(it),it.whatToShow,it.filter,Object.prototype.toString.call(it),Object.getOwnPropertyNames(it).length].join()}),
 fwd: t(()=>{fresh();return all(D.createNodeIterator(R))}),
 back: t(()=>{fresh();var it=D.createNodeIterator(R);all(it);return all(it,true)+'|'+st(it)}),
 show: t(()=>{fresh();return all(D.createNodeIterator(R,NodeFilter.SHOW_ELEMENT))+'|'+all(D.createNodeIterator(R,NodeFilter.SHOW_TEXT|NodeFilter.SHOW_COMMENT))}),
 filt: t(()=>{fresh();return all(D.createNodeIterator(R,1,function(n){return n.id==='a'?NodeFilter.FILTER_REJECT:NodeFilter.FILTER_ACCEPT}))+'|'+all(D.createNodeIterator(R,1,{acceptNode:function(n){return n.id==='b'?3:1}}))}),
 rmRef: t(()=>{fresh();var it=D.createNodeIterator(R);it.nextNode();it.nextNode();it.nextNode();var s1=st(it);D.getElementById('a').remove();return s1+'>'+st(it)+'>'+nm(it.nextNode())}),
 rmRefBefore: t(()=>{fresh();var it=D.createNodeIterator(R);it.nextNode();it.nextNode();it.nextNode();it.previousNode();var s1=st(it);D.getElementById('a').remove();return s1+'>'+st(it)+'>'+nm(it.nextNode())}),
 rmLastBefore: t(()=>{fresh();var it=D.createNodeIterator(R);all(it);all(it,true);all(it);it.previousNode();var s1=st(it);D.getElementById('c').remove();return s1+'>'+st(it)+'>'+nm(it.nextNode())}),
 innerHTML: t(()=>{fresh();var it=D.createNodeIterator(R);it.nextNode();it.nextNode();it.nextNode();R.innerHTML='<b>z</b>';return st(it)+'>'+nm(it.nextNode())}),
 moveRef: t(()=>{fresh();var it=D.createNodeIterator(R);it.nextNode();it.nextNode();var b=D.getElementById('b');var s1=st(it);D.body.appendChild(D.getElementById('a'));var r=s1+'>'+st(it)+'>'+nm(it.nextNode());fresh();return r}),
 rmRoot: t(()=>{fresh();var p=D.getElementById('a');var it=D.createNodeIterator(p);it.nextNode();p.remove();var r=st(it)+'>'+nm(it.nextNode());R.prepend(p);return r}),
 detach: t(()=>{var it=D.createNodeIterator(R);return String(it.detach())+','+nm(it.nextNode())}),
 badRoot: t(()=>D.createNodeIterator({})), noRoot: t(()=>D.createNodeIterator()), nullRoot: t(()=>D.createNodeIterator(null)),
 reentrant: t(()=>{fresh();var it;it=D.createNodeIterator(R,1,function(){it.nextNode();return 1});return nm(it.nextNode())}),
 brand: t(()=>Object.getOwnPropertyDescriptor(NodeIterator.prototype,'root').get.call({})),
 brandM: t(()=>NodeIterator.prototype.nextNode.call({})),
});
