// setHTMLUnsafe (js.md §15.10): run on Firefox 153.0esr (identity.md §3.19
// rig) over sethtml/page.html, its answers kept in firefox153.json.
var out = {};
var $ = function (id) { return document.getElementById(id); };
var t = function (k, f) { try { var v = f(); out[k] = v === undefined ? 'undefined' : v; } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var log = [];
customElements.define('x-el', class extends HTMLElement { constructor() { super(); log.push('ctor:' + this.childNodes.length + ':' + !!this.shadowRoot); } connectedCallback() { log.push('conn'); } });
t('shape', function () { return [typeof Element.prototype.setHTMLUnsafe, Element.prototype.setHTMLUnsafe.length, typeof ShadowRoot.prototype.setHTMLUnsafe, typeof Document.parseHTMLUnsafe, Object.prototype.hasOwnProperty.call(Element.prototype, 'setHTMLUnsafe')].join(' '); });
t('plain', function () { var r = $('t').setHTMLUnsafe('<b>new</b> text'); return r + ' ' + $('t').innerHTML; });
t('dsd', function () { $('t').setHTMLUnsafe('<div id=host><template shadowrootmode=open><i>shadow</i><slot></slot></template>light</div>'); var host = $('host'); return [!!host.shadowRoot, host.shadowRoot && host.shadowRoot.innerHTML, host.innerHTML, $('t').innerHTML].join(' | '); });
t('dsdClosed', function () { $('t').setHTMLUnsafe('<div id=c><template shadowrootmode=closed><i>x</i></template></div>'); return $('c').shadowRoot + ' ' + $('c').innerHTML + ' ' + $('t').getHTML({ serializableShadowRoots: true }); });
t('innerHTMLNo', function () { $('t').innerHTML = '<div id=n><template shadowrootmode=open>z</template></div>'; return $('n').shadowRoot + ' ' + $('n').innerHTML; });
t('shadowRoot', function () { var r = $('h').attachShadow({ mode: 'open' }); r.setHTMLUnsafe('<p id=sp>in<template shadowrootmode=open>deep</template></p>'); return [r.innerHTML, !!r.getElementById('sp').shadowRoot, r.getElementById('sp').shadowRoot && r.getElementById('sp').shadowRoot.innerHTML].join(' | '); });
t('template', function () { var tp = document.createElement('template'); tp.setHTMLUnsafe('<div id=q><template shadowrootmode=open>in tpl</template></div>'); var q = tp.content.firstChild; return tp.childNodes.length + ' ' + tp.content.childNodes.length + ' ' + !!(q && q.shadowRoot) + ' ' + tp.innerHTML; });
t('custom', function () { log.length = 0; $('t').setHTMLUnsafe('<x-el id=y><template shadowrootmode=open>s</template>kid</x-el>'); return log.join(',') + ' ' + !!$('y').shadowRoot; });
t('script', function () { window.ran = 0; $('t').setHTMLUnsafe('<script>window.ran=1<\/script>'); return window.ran; });
t('arg', function () { $('t').setHTMLUnsafe(5); return $('t').innerHTML; });
t('none', function () { return $('t').setHTMLUnsafe(); });
t('parse', function () { var d = Document.parseHTMLUnsafe('<title>T</title><div id=p><template shadowrootmode=open>sh</template></div>'); return [Object.prototype.toString.call(d), d.title, !!d.getElementById('p').shadowRoot, d === document, d.URL].join(' '); });
resolve(out);
