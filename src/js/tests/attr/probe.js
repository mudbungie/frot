// Attribute-list probe (js.md §15): run on Firefox 153.0esr (identity.md §3.19 rig)
// over tests/../attr/page.html, its answers kept verbatim in firefox153.json.
var out = {};
function T(k, f) { try { var v = f(); out[k] = v === undefined ? 'undefined' : v; } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } }
var d = document.getElementById('d');
function own(o){ return Object.getOwnPropertyNames(o).join(' '); }
T('names', ()=>JSON.stringify(d.getAttributeNames()) + ' ' + Array.isArray(d.getAttributeNames()));
T('hasAttrs', ()=>d.hasAttributes()+' '+document.createElement('i').hasAttributes());
T('toggle', ()=>{ var e=document.createElement('i'); var a=[e.toggleAttribute('hidden'), e.outerHTML, e.toggleAttribute('hidden'), e.outerHTML, e.toggleAttribute('x', true), e.toggleAttribute('x', true), e.outerHTML, e.toggleAttribute('x', false), e.toggleAttribute('y', false), e.outerHTML, e.toggleAttribute('X'), e.outerHTML]; return JSON.stringify(a); });
T('toggleBad', ()=>document.createElement('i').toggleAttribute('1a'));
T('toggleNone', ()=>document.createElement('i').toggleAttribute());
T('localName', ()=>d.localName+' '+document.createElement('X-Y').localName+' '+document.createElementNS('http://www.w3.org/2000/svg','svg').localName);
T('attrs', ()=>{ var a=d.attributes; return Object.prototype.toString.call(a)+' '+a.length+' '+a[0].name+'='+a[0].value+' '+a.item(1).name+' '+(a.getNamedItem('data-x')||{}).value+' '+a.getNamedItem('zz')+' '+(a===d.attributes)+' '+Object.prototype.toString.call(a[0]); });
T('attrIter', ()=>Array.from(d.attributes).map(x=>x.name).join(',')+' '+Array.from(d.attributes, x=>x.localName+'/'+x.nodeName+'/'+x.nodeType+'/'+x.specified+'/'+(x.ownerElement===d)+'/'+x.namespaceURI+'/'+x.prefix).join(','));
T('attrLive', ()=>{ var a=d.attributes; d.setAttribute('z','9'); var n=a.length; d.removeAttribute('z'); return n+' '+a.length; });
T('attrSet', ()=>{ var at=d.attributes[0]; at.value='q'; return d.getAttribute('a')+' '+at.value+' '+at.textContent+' '+at.nodeValue; });
T('attrStale', ()=>{ var at=d.attributes.getNamedItem('data-x'); d.removeAttribute('data-x'); var r=at.value+' '+at.ownerElement; d.setAttribute('data-x','y'); return r; });
T('nnmProto', ()=>own(NamedNodeMap.prototype));
T('attrProto', ()=>own(Attr.prototype)+' | '+(Object.getPrototypeOf(Attr.prototype)===Node.prototype));
T('nnmKeys', ()=>Object.keys(d.attributes).join(',')+' | '+Object.getOwnPropertyNames(d.attributes).join(','));
T('attrOwn', ()=>Object.getOwnPropertyNames(d.attributes[0]).length);
T('attrSame', ()=>d.attributes[0]===d.attributes[0]);
T('elemOrder', ()=>own(Element.prototype).split(' ').filter(k=>['getAttributeNames','getAttribute','toggleAttribute','hasAttributes','localName','attributes','tagName'].indexOf(k)>=0).join(' '));
resolve(out);
