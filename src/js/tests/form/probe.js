// Forms probe (bl-09c8): run on Firefox 153.0esr (identity.md §3.19 rig) over
// form/page.html; its answers kept verbatim in firefox153.json. URLs are
// reported without the page's origin, which differs between the two runs.
function t(f){try{var v=f();v=v===undefined?'undef':v===null?'null':String(v)}catch(e){v=e.name+':'+e.message}return v.split(location.origin).join('')}
var f=document.getElementById('f'), D=document;
var r={
 URL:t(()=>D.URL), docURI:t(()=>D.documentURI), urlDesc:t(()=>Object.keys(Object.getOwnPropertyDescriptor(Document.prototype,'URL')).join('/')),
 title:t(()=>D.body.title), titleNone:t(()=>D.head.title), titleSet:t(()=>{var e=D.createElement('p');e.title=5;return e.getAttribute('title')}),
 svgTitle:t(()=>typeof D.createElementNS('http://www.w3.org/2000/svg','svg').title),
 forms:t(()=>D.forms.length+'|'+Object.prototype.toString.call(D.forms)+'|'+(D.forms===D.forms)+'|'+D.forms[0].id+'|'+D.forms.ff.id+'|'+D.forms.namedItem('g').id),
 formsLive:t(()=>{var c=D.forms,n=c.length;D.body.appendChild(D.createElement('form'));var m=c.length;D.body.lastChild.remove();return n+'>'+m}),
 elements:t(()=>{var e=f.elements;return e.length+'|'+Object.prototype.toString.call(e)+'|'+[].map.call(e,x=>x.name||x.id).join('/')+'|'+(e===f.elements)+'|'+f.length}),
 elNamed:t(()=>f.elements.qq.id+'|'+f.elements.namedItem('os').id+'|'+f.elements['im']),
 action:t(()=>f.action+'|'+D.getElementById('g').action+'|'+D.getElementById('h').action+'|'+D.getElementById('k').action),
 actionSet:t(()=>{var g=D.getElementById('g');g.action='z';return g.getAttribute('action')+'|'+g.action}),
 controlForm:t(()=>[D.getElementById('q').form.id, D.getElementById('outside').form.id, D.getElementById('loose').form, D.getElementById('fs').form.id, D.querySelector('select').form.id, D.querySelector('label').form, D.querySelector('output').form.id, D.querySelector('object').form.id].join('/')),
 formNamed:t(()=>typeof f.qq+'|'+(f.qq&&f.qq.id)+'|'+typeof f.pic+'|'+typeof f.os+'|'+typeof f.im),
 docNamed:t(()=>typeof D.ff+'|'+typeof D.pic+'|'+typeof D.q+'|'+typeof D.f),
 winNamed:t(()=>typeof window.f+'|'+typeof window.q+'|'+typeof window.ff+'|'+typeof window.qq+'|'+typeof window.pic+'|'+(window.loose&&window.loose.id)),
 fieldsetEls:t(()=>D.getElementById('fs').elements.length+'|'+Object.prototype.toString.call(D.getElementById('fs').elements)),
 hfcc:t(()=>Object.getOwnPropertyNames(HTMLFormControlsCollection.prototype).join(' ')+'|'+Object.getPrototypeOf(HTMLFormControlsCollection.prototype).constructor.name),
 hc:t(()=>Object.getOwnPropertyNames(HTMLCollection.prototype).join(' ')),
 rnl:t(()=>{var x=D.createElement('form');x.innerHTML='<input name=r type=radio value=1><input name=r type=radio value=2 checked>';var n=x.elements.r;return Object.prototype.toString.call(n)+'|'+n.length+'|'+n.value+'|'+Object.getOwnPropertyNames(RadioNodeList.prototype).join(' ')}),
};
resolve(r);
