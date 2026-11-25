// Constructable stylesheet probe (js.md §15): run on Firefox 153.0esr (identity.md
// §3.19 rig) over tests/../cssom/page.html, its answers kept verbatim in firefox153.json.
var out = {};
function T(k, f) { try { var v = f(); out[k] = v === undefined ? 'undefined' : v; } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } }
function own(o){ return Object.getOwnPropertyNames(o).join(' '); }
var s = new CSSStyleSheet();
T('new', ()=>[Object.prototype.toString.call(s), s.cssRules.length, s.type, s.href, s.ownerNode, s.title, s.disabled, s.media.length, Object.prototype.toString.call(s.cssRules)].join(' '));
T('replaceSync', ()=>{ var x=s.replaceSync('p { color: red } .a, .b { display: none } @media screen { i { x: y } }'); return x+' '+s.cssRules.length+' '+Array.from(s.cssRules, function(c){return c.cssText}).join(' | '); });
T('replaceRet', ()=>Object.prototype.toString.call(s.replace('b{}')));
T('insert', ()=>{ var t=new CSSStyleSheet(); t.replaceSync('a{} b{}'); var i=t.insertRule('c { color: red }'); var j=t.insertRule('d{}', 3); t.deleteRule(0); return i+' '+j+' '+Array.from(t.cssRules, function(c){return c.cssText}).join(' | '); });
T('insertBad', ()=>{ var t=new CSSStyleSheet(); t.insertRule('a{}', 5); });
T('importReject', ()=>{ var t=new CSSStyleSheet(); t.replaceSync('@import url(x.css); a{}'); return t.cssRules.length; });
T('docAdopted', ()=>{ var a=document.adoptedStyleSheets; return Object.prototype.toString.call(a)+' '+Array.isArray(a)+' '+a.length+' '+(a===document.adoptedStyleSheets); });
T('docSet', ()=>{ var t=new CSSStyleSheet(); t.replaceSync('#x{display:none}'); document.adoptedStyleSheets=[t]; return document.adoptedStyleSheets.length+' '+(document.adoptedStyleSheets[0]===t)+' '+getComputedStyle(document.getElementById('x')).display; });
T('docPush', ()=>{ var t=new CSSStyleSheet(); t.replaceSync('#y{display:none}'); document.adoptedStyleSheets.push(t); return document.adoptedStyleSheets.length+' '+getComputedStyle(document.getElementById('y')).display; });
T('docLive', ()=>{ var t=document.adoptedStyleSheets[0]; t.replaceSync('#x{display:inline-block}'); return getComputedStyle(document.getElementById('x')).display; });
T('setBad', ()=>{ document.adoptedStyleSheets = [5]; });
T('setNotArr', ()=>{ document.adoptedStyleSheets = 5; });
T('setNonConstructed', ()=>{ document.adoptedStyleSheets = [document.styleSheets[0]]; });
T('srAdopted', ()=>{ var d=document.createElement('div'); document.body.appendChild(d); var sr=d.attachShadow({mode:'open'}); sr.innerHTML='<p>q</p>'; var t=new CSSStyleSheet(); t.replaceSync('p{display:none}'); sr.adoptedStyleSheets=[t]; return getComputedStyle(sr.firstChild).display+' '+sr.adoptedStyleSheets.length; });
T('proto', ()=>own(CSSStyleSheet.prototype)+' || '+own(StyleSheet.prototype)+' || '+(Object.getPrototypeOf(CSSStyleSheet.prototype)===StyleSheet.prototype));
T('ruleProto', ()=>{ var t=new CSSStyleSheet(); t.replaceSync('p{}'); var c=t.cssRules[0]; return Object.prototype.toString.call(c)+' '+own(Object.getPrototypeOf(c))+' || '+Object.prototype.toString.call(t.cssRules)+' '+own(Object.getPrototypeOf(t.cssRules)); });
T('docDesc', ()=>{ var d=Object.getOwnPropertyDescriptor(Document.prototype,'adoptedStyleSheets'); return typeof d.get+' '+typeof d.set+' '+d.enumerable; });
resolve(out);
