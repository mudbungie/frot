// Window probe (bl-226d): run on Firefox 153.0esr (identity.md §3.19 rig) as a
// page script over window/page.html; its answers kept verbatim in
// firefox153.json.
function t(f){try{var v=f();return v===undefined?'undef':v===null?'null':String(v)}catch(e){return 'THROW '+e.name+':'+e.message}}
var W=window, P=Object.getPrototypeOf(W), PP=Object.getPrototypeOf(P), PPP=Object.getPrototypeOf(PP);
function d(o,k){var x=Object.getOwnPropertyDescriptor(o,k);if(!x)return 'none';return (x.get?'g':'')+(x.set?'s':'')+('value' in x?(typeof x.value==='function'?'f':'v'):'')+(x.enumerable?'e':'')+(x.configurable?'c':'')+(x.writable?'w':'')}
resolve({
 chain: t(()=>[P===Window.prototype, Object.prototype.toString.call(P), Object.prototype.toString.call(PP), PPP===EventTarget.prototype, Object.getPrototypeOf(PPP)===Object.prototype].join()),
 ppNames: t(()=>Object.getOwnPropertyNames(PP).join(' ')+'|'+String(PP.constructor)+'|'+typeof WindowProperties),
 protoNames: t(()=>Object.getOwnPropertyNames(Window.prototype).join(' ')),
 protoSyms: t(()=>Object.getOwnPropertySymbols(Window.prototype).map(String).join(' ')),
 ctor: t(()=>[Window.length, Window.name, typeof Window, d(W,'Window'), Object.getPrototypeOf(Window)===EventTarget, String(W), W.constructor===Window, W instanceof Window, W instanceof EventTarget].join()),
 newW: t(()=>new Window()),
 callW: t(()=>Window()),
 tag: t(()=>[Object.prototype.toString.call(W), W[Symbol.toStringTag], d(Window.prototype, Symbol.toStringTag), d(PP, Symbol.toStringTag)].join()),
 aelOwn: t(()=>[d(W,'addEventListener'), d(EventTarget.prototype,'addEventListener'), W.addEventListener===EventTarget.prototype.addEventListener].join()),
 named: t(()=>[typeof W.f, typeof W.q, typeof W.ff, typeof W.qq, typeof W.pic, typeof W.loose, typeof W.im, typeof W.fs, typeof W.s].join()),
 namedDesc: t(()=>[d(W,'f'), d(PP,'f'), 'f' in W, Object.keys(W).indexOf('f'), Object.getOwnPropertyNames(PP).indexOf('f'), PP.f===document.getElementById('f')].join()),
 namedBare: t(()=>[typeof f, typeof loose, typeof pic, f===document.getElementById('f')].join()),
 namedDup: t(()=>{var a=document.createElement('p');a.id='dup';var b=document.createElement('p');b.id='dup';document.body.append(a,b);var r=W.dup;var s=Object.prototype.toString.call(r)+'|'+r.length;a.remove();b.remove();return s+'|'+typeof W.dup}),
 namedLive: t(()=>{var a=document.createElement('p');a.id='zz9';var r=[typeof W.zz9];document.body.append(a);r.push(W.zz9===a);a.remove();r.push(typeof W.zz9);return r.join()}),
 shadow: t(()=>{var a=document.createElement('p');a.id='alert';document.body.append(a);var r=typeof W.alert;a.remove();var b=document.createElement('p');b.id='myvar';document.body.append(b);W.myvar=5;var r2=W.myvar;delete W.myvar;var r3=W.myvar===b;b.remove();return r+','+r2+','+r3}),
 iframeNamed: t(()=>{var i=document.createElement('iframe');i.name='frm';document.body.append(i);var r=[W.frm===i.contentWindow, typeof W.frm];i.remove();return r.join()}),
 setPP: t(()=>{try{Object.setPrototypeOf(PP,null);return 'set'}catch(e){return e.name}}),
 defPP: t(()=>{try{Object.defineProperty(PP,'xx',{value:1});return 'ok'}catch(e){return e.name+':'+e.message}}),
 globalThisSame: t(()=>[globalThis===W, self===W, W.window===W, frames===W, d(W,'window'), d(W,'self'), d(W,'document'), d(W,'location'), d(W,'top')].join()),
});
