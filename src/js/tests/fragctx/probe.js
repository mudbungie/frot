// Fragment-parsing context probe (bl-538f): run on Firefox 153.0esr (identity.md
// §3.19 rig) as a page script over fragctx/page.html; its answers kept
// verbatim in firefox153.json.
function t(f){try{var v=f();return v===undefined?'undef':v===null?'null':String(v)}catch(e){return 'THROW '+e.name+':'+e.message}}
var D=document, M='p<b>q</b>r<!--c-->s&amp;</x>';
function kids(el){return [].map.call(el.childNodes,function(n){return n.nodeType===1?n.localName:n.nodeType===3?'#'+n.data:n.nodeType===8?'!'+n.data:n.nodeType}).join('|')}
function set(tag){var e=D.createElement(tag);e.innerHTML=M;return kids(e)+' => '+e.innerHTML}
resolve({
 title: t(()=>{var e=D.querySelector('title');e.innerHTML=M;return kids(e)+' => '+D.title}),
 textarea: t(()=>set('textarea')), style: t(()=>set('style')), script: t(()=>set('script')), xmp: t(()=>set('xmp')), iframe: t(()=>set('iframe')),
 noembed: t(()=>set('noembed')), noframes: t(()=>set('noframes')), noscript: t(()=>set('noscript')), plaintext: t(()=>set('plaintext')), div: t(()=>set('div')),
 tbody: t(()=>{var e=D.getElementById('tbo');e.innerHTML='<tr><td>1</td></tr>';return kids(e)+' / '+kids(e.firstChild)}),
 table: t(()=>{var e=D.getElementById('tb');e.innerHTML='<tr><td>1</td></tr>';return kids(e)+' / '+kids(e.firstChild)}),
 select: t(()=>{var e=D.getElementById('se');e.innerHTML='<option>a</option><div>b</div><option>c';return kids(e)}),
 divTr: t(()=>{var e=D.createElement('div');e.innerHTML='<tr><td>1</td></tr>';return kids(e)}),
 outer: t(()=>{var p=D.getElementById('p');p.outerHTML='<td>1</td><b>2</b>';return kids(D.getElementById('d'))}),
 outerTitle: t(()=>{var e=D.querySelector('title');var s=D.createElement('span');e.appendChild(s);s.outerHTML='a<b>c</b>';return kids(e)}),
 tmpl: t(()=>{var e=D.createElement('template');e.innerHTML='<tr><td>1</td></tr>';return kids(e.content)}),
 html: t(()=>{var e=D.createElement('html');e.innerHTML='<head><title>x</title></head><p>y';return kids(e)}),
 insAdj: t(()=>{var e=D.createElement('textarea');e.insertAdjacentHTML('beforeend','<b>z</b>');return kids(e)}),

 iahAll: t(()=>{var d=D.createElement('div'),p=D.createElement('p');d.append(p);p.insertAdjacentHTML('beforeBegin','<i>1</i>');p.insertAdjacentHTML('AFTERBEGIN','<b>2</b>');p.insertAdjacentHTML('beforeend','3');var r=p.insertAdjacentHTML('afterend','<u>4</u><!--x-->');return kids(d)+' / '+kids(p)+' / '+r}),
 iahBad: t(()=>D.createElement('p').insertAdjacentHTML('middle','x')),
 iahDetached: t(()=>D.createElement('p').insertAdjacentHTML('beforebegin','x')),
 iahDetachedAfter: t(()=>D.createElement('p').insertAdjacentHTML('afterend','x')),
 iahRoot: t(()=>D.documentElement.insertAdjacentHTML('afterend','x')),
 iahNone: t(()=>D.createElement('p').insertAdjacentHTML('beforeend')),
 iahOne: t(()=>D.createElement('p').insertAdjacentHTML()),
 iahTr: t(()=>{var tb=D.createElement('table');var tr=D.createElement('tr');tb.append(tr);tr.insertAdjacentHTML('afterend','<tr><td>9');return kids(tb)}),
 iahHtml: t(()=>{var h=D.createElement('html');h.insertAdjacentHTML('afterbegin','<td>x</td><p>y');return kids(h)}),
 iahFrag: t(()=>{var f=D.createDocumentFragment(),p=D.createElement('p');f.append(p);p.insertAdjacentHTML('beforebegin','<td>z</td>q');return kids(f)}),
 iahLen: t(()=>Element.prototype.insertAdjacentHTML.length),
 iahScript: t(()=>{window.RAN=0;var d=D.createElement('div');D.body.append(d);d.insertAdjacentHTML('beforeend','<script>RAN=1<\/script>');d.remove();return RAN}),
});
