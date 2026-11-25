// Custom elements probe (js.md §15): run on Firefox 153.0esr (identity.md §3.19
// rig) over tests/../customel/page.html, its answers kept verbatim in
// firefox153.json. Synchronous, then one microtask for the promise answers.
var out = {};
var log = [];
var t = function (k, f) { try { var v = f(); out[k] = v === undefined ? 'undefined' : v; } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var take = function () { return log.splice(0).join(' | '); };
var nm = function (e) { return e.id || e.localName; };
var $ = function (id) { return document.getElementById(id); };
class XA extends HTMLElement {
  static get observedAttributes() { log.push('observed'); return ['foo', 'bar']; }
  constructor() { super(); log.push('ctor ' + nm(this) + ' conn=' + this.isConnected + ' kids=' + this.childNodes.length + ' attrs=' + this.attributes.length); }
  connectedCallback() { log.push('conn ' + nm(this)); }
  disconnectedCallback() { log.push('disc ' + nm(this)); }
  adoptedCallback() { log.push('adopt ' + nm(this)); }
  attributeChangedCallback(n, o, v) { log.push('attr ' + nm(this) + ' ' + n + ' ' + o + ' ' + v); }
}
t('before', function () { return document.querySelectorAll(':defined').length + ' ' + $('a1').matches(':defined') + ' ' + (Object.getPrototypeOf($('a1')) === HTMLElement.prototype); });
t('getUndef', function () { return customElements.get('x-a') + ' ' + customElements.getName(XA); });
var wd = customElements.whenDefined('x-a');
wd.then(function (c) { log.push('whenDefined ' + (c === XA)); });
t('wdSame', function () { return customElements.whenDefined('x-a') === wd; });
t('define', function () { customElements.define('x-a', XA); return take(); });
t('after', function () { return (Object.getPrototypeOf($('a1')) === XA.prototype) + ' ' + $('a1').matches(':defined') + ' ' + document.querySelectorAll(':not(:defined)').length; });
t('get', function () { return (customElements.get('x-a') === XA) + ' ' + customElements.getName(XA); });
t('templateNot', function () { return Object.getPrototypeOf($('t').content.firstChild) === XA.prototype; });
t('create', function () { var e = document.createElement('x-a'); e.id = 'c1'; return (e instanceof XA) + ' ' + take(); });
t('new', function () { var e = new XA(); return e.localName + ' ' + e.isConnected + ' ' + e.matches(':defined') + ' ' + take(); });
t('insertRemove', function () { var e = document.createElement('x-a'); e.id = 'c2'; take(); document.body.appendChild(e); var a = take(); e.remove(); return a + ' || ' + take(); });
t('attrs', function () { var e = $('a1'); e.setAttribute('foo', '2'); e.setAttribute('baz', '3'); e.removeAttribute('foo'); e.setAttribute('bar', 'b'); e.setAttribute('bar', 'b'); e.removeAttribute('nope'); return take(); });
t('innerDetached', function () { var d = document.createElement('div'); d.innerHTML = '<x-a id=i1 foo=9></x-a>'; var a = take() + ' proto=' + (d.firstChild instanceof XA); document.body.appendChild(d); a += ' || ' + take(); d.remove(); take(); return a; });
t('innerConnected', function () { $('d').innerHTML = '<x-a id=i2 foo=8><x-a id=i3></x-a></x-a>'; return take() + ' proto=' + ($('d').firstChild instanceof XA); });
t('importNode', function () { var f = document.importNode($('t').content, true); return take() + ' proto=' + (f.firstChild instanceof XA); });
t('cloneNode', function () { var c = $('a1').cloneNode(true); return take() + ' proto=' + (c instanceof XA); });
t('move', function () { var e = $('i3'); $('d').appendChild(e); return take(); });
t('again', function () { customElements.define('x-a', class extends HTMLElement {}); });
t('sameCtor', function () { customElements.define('x-b', XA); });
t('badName', function () { customElements.define('xa', class extends HTMLElement {}); });
t('upper', function () { customElements.define('X-c', class extends HTMLElement {}); });
t('reserved', function () { customElements.define('font-face', class extends HTMLElement {}); });
t('bound', function () { customElements.define('x-d', function () {}.bind(null)); });
t('arrow', function () { customElements.define('x-d', function () {}.prototype.constructor && (() => 1)); });
t('noArgs', function () { customElements.define(); });
t('upgradeFn', function () { var d = document.createElement('div'); customElements.define('x-v', class extends HTMLElement { constructor() { super(); log.push('vctor'); } }); d.innerHTML = '<x-v></x-v>'; var a = take(); customElements.upgrade(d); return a + ' || ' + take() + ' ' + d.firstChild.matches(':defined'); });
t('upgradeLate', function () { var d = document.createElement('div'); d.innerHTML = '<x-w></x-w>'; customElements.define('x-w', class extends HTMLElement { constructor() { super(); log.push('wctor'); } }); customElements.upgrade(d); return take() + ' ' + d.firstChild.matches(':defined'); });
t('newUndefined', function () { class Q extends HTMLElement {} return new Q(); });
t('newHTMLElement', function () { return new HTMLElement(); });
t('ctorThrows', function () { customElements.define('x-t', class extends HTMLElement { constructor() { super(); throw new Error('boom'); } }); var e = document.createElement('x-t'); return Object.prototype.toString.call(e) + ' ' + e.localName + ' ' + e.matches(':defined'); });
t('ctorOther', function () { customElements.define('x-r', class extends HTMLElement { constructor() { super(); return document.createElement('div'); } }); var e = document.createElement('x-r'); return e.localName + ' ' + Object.prototype.toString.call(e); });
t('definedPc', function () { return $('a1').matches(':defined') + ' ' + document.createElement('x-zz').matches(':defined') + ' ' + document.createElement('div').matches(':defined'); });
t('builtin', function () {
  class XB extends HTMLButtonElement { constructor() { super(); log.push('xb ctor ' + this.localName); } connectedCallback() { log.push('xb conn'); } }
  customElements.define('x-btn', XB, { extends: 'button' });
  var b1 = $('b1');
  var a = take() + ' proto=' + (b1 instanceof XB) + ' def=' + b1.matches(':defined');
  var c = document.createElement('button', { is: 'x-btn' });
  a += ' || ' + take() + ' ' + (c instanceof XB) + ' attr=' + c.getAttribute('is') + ' ' + c.outerHTML;
  var n = new XB();
  return a + ' || ' + n.localName + ' ' + n.outerHTML + ' ' + take();
});
t('wrongIface', function () { class XC extends HTMLElement {} customElements.define('x-c2', XC, { extends: 'div' }); return new XC().localName; });
t('extendsCustom', function () { customElements.define('x-c3', class extends HTMLElement {}, { extends: 'x-y' }); });
t('extendsUnknown', function () { customElements.define('x-c4', class extends HTMLElement {}, { extends: 'bogus' }); });
t('isUndefined', function () { var c = document.createElement('div', { is: 'x-nope' }); return c.outerHTML + ' ' + c.getAttribute('is'); });
t('isOnCustom', function () { var c = document.createElement('x-a', { is: 'x-btn' }); return c.outerHTML + ' ' + (c instanceof XA); });
t('badCallback', function () { var C = class extends HTMLElement {}; C.prototype.connectedCallback = 5; customElements.define('x-g', C); });
t('protoNotObject', function () { var f = function () {}; f.prototype = 5; customElements.define('x-h', f); });
t('reentrant', function () { customElements.define('x-e', class extends HTMLElement { static get observedAttributes() { customElements.define('x-f', class extends HTMLElement {}); return []; } attributeChangedCallback() {} }); });
t('wdBad', function () { var p = customElements.whenDefined('nope'); p.then(null, function (e) { log.push('wd ' + e.name + ': ' + e.message); }); return Object.prototype.toString.call(p); });
t('getNoArg', function () { return customElements.get(); });
t('upgradeNoArg', function () { customElements.upgrade(); });
t('shape', function () { return Object.getOwnPropertyNames(CustomElementRegistry.prototype).join(' ') + ' | ' + Object.prototype.toString.call(customElements) + ' ' + Object.keys(Object.getOwnPropertyDescriptor(window, 'customElements')).join(',') + ' ' + HTMLElement.length + ' ' + customElements.define.length + ' ' + customElements.whenDefined.length; });
t('newRegistry', function () { return new CustomElementRegistry(); });
Promise.resolve().then(function () { out.late = take(); resolve(out); });
