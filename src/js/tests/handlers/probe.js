// Event handler probe (bl-93c8): run on Firefox 153.0esr (identity.md §3.19 rig)
// over handlers/page.html, its answers kept verbatim in firefox153.json.
var out = {};
var t = function (k, f) { try { out[k] = f(); } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
function d(o, k) { var x = Object.getOwnPropertyDescriptor(o, k); if (!x) return 'none'; return (x.get ? 'g' : '') + (x.set ? 's' : '') + ('value' in x ? 'v' : '') + (x.enumerable ? 'e' : '') + (x.configurable ? 'c' : '') + (x.get ? ':' + x.get.name + ':' + x.set.name + ':' + x.get.length + x.set.length : ''); }
var errs = [];
window.addEventListener('error', function (e) { errs.push((e.constructor && e.constructor.name) + ':' + e.message + ':' + (e.error && e.error.name) + ':' + e.lineno); e.preventDefault(); });
function E() { var r = errs.join(' '); errs = []; return r; }
function ev(type, init) { return new Event(type, init); }
var host = document.getElementById('host');
t('desc', function () { return [d(HTMLElement.prototype, 'onclick'), d(Document.prototype, 'onclick'), d(window, 'onload'), d(Object.getPrototypeOf(window), 'onload'), d(window, 'onerror'), d(HTMLBodyElement.prototype, 'onload'), d(HTMLElement.prototype, 'onload'), d(SVGElement.prototype, 'onclick'), d(Element.prototype, 'onclick')].join(' | '); });
t('getterStr', function () { var x = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'onclick'); return x.get.toString() + '/' + x.set.toString(); });
t('defaults', function () { var el = document.createElement('div'); return [el.onclick, el.onload, el.onerror, document.onclick, window.onmessage, typeof el.onfoo, 'onclick' in el, el.hasOwnProperty('onclick')].join(); });
t('setValues', function () { var el = document.createElement('div'); var r = []; [function () {}, {}, 5, 'str', true, null, undefined, Symbol.iterator].forEach(function (v) { try { el.onclick = v; r.push(el.onclick === v ? 'same' : el.onclick === null ? 'null' : typeof el.onclick); } catch (e) { r.push(e.name); } }); return r.join(); });
t('objHandler', function () { var el = document.createElement('div'); el.onclick = { handleEvent: function () { r = 'called'; } }; var r = 'no'; el.dispatchEvent(ev('click')); return r + ';' + E(); });
t('order', function () {
  var el = document.createElement('div'), log = [];
  function L(n) { return function () { log.push(n); }; }
  el.addEventListener('x', L('l1')); el.onx = 1;
  var p = 'onclick';
  el.addEventListener('click', L('l1')); el.onclick = L('h1'); el.addEventListener('click', L('l2')); el.onclick = L('h2');
  el.dispatchEvent(ev('click')); log.push('|');
  el.onclick = null; el.onclick = L('h3'); el.dispatchEvent(ev('click')); log.push('|');
  el.onclick = 5; el.dispatchEvent(ev('click')); log.push('|');
  el.onclick = L('h4'); el.setAttribute('onclick', "log.push('a1')"); el.dispatchEvent(ev('click')); log.push('|');
  el.removeAttribute('onclick'); el.dispatchEvent(ev('click')); log.push(String(el.onclick));
  return log.join(' ');
});
t('attrOrder', function () {
  var el = document.createElement('div'), log = [];
  window.log2 = log;
  el.addEventListener('click', function () { log.push('l1'); });
  el.setAttribute('onclick', "log2.push('a')");
  el.addEventListener('click', function () { log.push('l2'); });
  el.dispatchEvent(ev('click'));
  el.setAttribute('onclick', "log2.push('b')");
  el.dispatchEvent(ev('click'));
  return log.join(' ');
});
t('compiled', function () { var el = document.createElement('div'); el.setAttribute('onclick', 'return event.type + this.tagName'); var f = el.onclick; return [typeof f, f.name, f.length, JSON.stringify(f.toString()), f === el.onclick, f.call(el, ev('q'))].join('|'); });
t('compiledErr', function () { var el = document.createElement('div'); el.setAttribute('onerror', 'return event.type'); var f = el.onerror; return [f.length, JSON.stringify(f.toString())].join('|'); });
t('bodyErrAttr', function () { var b = document.createElement('body'); b.setAttribute('onerror', 'return 1'); var f = b.onerror; return [typeof f, f && f.length, f && JSON.stringify(f.toString())].join('|'); });
t('attrNoIdl', function () { var el = document.createElement('div'); el.setAttribute('onclick', 'x'); el.onclick = null; return [el.getAttribute('onclick'), el.onclick].join(); });
t('idlNoAttr', function () { var el = document.createElement('div'); el.onclick = function () {}; return el.getAttribute('onclick') + ',' + el.hasAttribute('onclick'); });
t('scope', function () {
  var input = document.getElementById('q');
  window.RES = null;
  input.dispatchEvent(ev('click'));
  return window.RES;
});
t('scopeOuter', function () { var b = document.getElementById('nb'); window.RES = null; b.dispatchEvent(ev('click')); return window.RES; });
t('returnFalse', function () { var el = document.createElement('div'); el.setAttribute('onclick', 'return false'); var e1 = ev('click', { cancelable: true }); var r1 = el.dispatchEvent(e1); el.onclick = function () { return 0; }; var e2 = ev('click', { cancelable: true }); var r2 = el.dispatchEvent(e2); el.onclick = function () { return true; }; var e3 = ev('click', { cancelable: true }); el.dispatchEvent(e3); el.onmouseover = function () { return false; }; var e4 = ev('mouseover', { cancelable: true }); el.dispatchEvent(e4); el.onerror = function () { return false; }; var e5 = ev('error', { cancelable: true }); el.dispatchEvent(e5); el.onerror = function () { return true; }; var e6 = ev('error', { cancelable: true }); el.dispatchEvent(e6); return [r1, e1.defaultPrevented, r2, e3.defaultPrevented, e4.defaultPrevented, e5.defaultPrevented, e6.defaultPrevented].join(); });
t('syntaxErr', function () { var el = document.createElement('div'); el.setAttribute('onclick', 'return ((('); var r = [String(el.onclick), E()]; el.dispatchEvent(ev('click')); r.push(E()); return r.join(';'); });
t('syntaxErrDispatch', function () { var el = document.createElement('div'); el.setAttribute('onclick', 'return ((('); var n = 0; el.addEventListener('click', function () { n++; }); el.dispatchEvent(ev('click')); var r = [n, E()]; el.dispatchEvent(ev('click')); r.push(n, E()); return r.join(';'); });
t('throwing', function () { var el = document.createElement('div'); el.setAttribute('onclick', "throw new RangeError('rr')"); el.dispatchEvent(ev('click')); return E(); });
t('throwingNonError', function () { var el = document.createElement('div'); el.setAttribute('onclick', "throw 'ss'"); el.dispatchEvent(ev('click')); return E(); });
t('bodyFwd', function () { var b = document.body; var f = function () {}; var r = [b.onload === window.onload, String(window.onload)]; b.onload = f; r.push(window.onload === f, b.onload === f); window.onload = null; r.push(b.onload); var nb = document.createElement('body'); nb.onload = f; r.push(window.onload === f, nb.onload === f); window.onload = null; nb.onclick = f; r.push(nb.onclick === f); var fs = document.createElement('frameset'); r.push(d(Object.getPrototypeOf(fs), 'onload') + ',' + d(HTMLBodyElement.prototype, 'onclick')); return r.join(); });
t('bodyAttr', function () { var log = []; window.LOGB = log; document.body.setAttribute('onmessage', 'LOGB.push(event.type, this === window)'); var r = [typeof window.onmessage, window.onmessage === document.body.onmessage]; window.dispatchEvent(ev('message')); document.body.removeAttribute('onmessage'); r.push(window.onmessage, log.join()); return r.join(); });
t('bodyOnclickAttr', function () { var log = []; window.LOGC = log; document.body.setAttribute('onclick', 'LOGC.push(this === document.body)'); document.body.dispatchEvent(ev('click')); document.body.removeAttribute('onclick'); return log.join() + typeof window.onclick + String(window.onclick); });
t('winOnerror', function () {
  var log = [];
  window.onerror = function (m, f, l, c, e) { log.push('on:' + [typeof m, m, typeof f, l, c, e && e.message, this === window, arguments.length].join('/')); return true; };
  window.addEventListener('error', function (e) { log.push('L:' + e.defaultPrevented); });
  var el = document.createElement('div');
  el.addEventListener('x', function () { throw new Error('w1'); });
  el.dispatchEvent(ev('x'));
  window.dispatchEvent(ev('error'));
  var plain = ev('error', { cancelable: true });
  window.onerror = function (e) { log.push('plain:' + (e === plain) + arguments.length); return false; };
  window.dispatchEvent(plain);
  log.push(plain.defaultPrevented);
  var ee = new ErrorEvent('error', { message: 'mm', cancelable: true });
  window.onerror = function (m) { log.push('ee:' + m + arguments.length); return false; };
  window.dispatchEvent(ee);
  log.push(ee.defaultPrevented);
  window.onerror = null;
  E();
  return log.join(' ');
});
t('winOnerrorAttr', function () { var b = document.createElement('body'); b.setAttribute('onerror', 'return 1'); var r = typeof window.onerror; document.body.setAttribute('onerror', 'window.OE = [event, source, lineno, colno, error && error.message, typeof arguments[0]].join("/"); return true'); var f = window.onerror; var el = document.createElement('div'); el.addEventListener('y', function () { throw new Error('w2'); }); el.dispatchEvent(ev('y')); document.body.removeAttribute('onerror'); return r + ';' + f.length + ';' + JSON.stringify(f.toString()) + ';' + window.OE + ';' + E(); });
t('docHandler', function () { var log = []; document.onclick = function (e) { log.push(this === document, e.eventPhase); }; host.dispatchEvent(ev('click', { bubbles: true })); document.onclick = null; return log.join(); });
t('docAttr', function () { document.documentElement.setAttribute('onclick', 'window.HTMLC = this.tagName'); host.dispatchEvent(ev('click', { bubbles: true })); document.documentElement.removeAttribute('onclick'); return window.HTMLC + ',' + document.onclick; });
t('innerHTML', function () { host.innerHTML = '<span id=sp onclick="window.SP = this.id + event.type">s</span>'; document.getElementById('sp').dispatchEvent(ev('click')); return window.SP; });
t('parsed', function () { var log = window.PLOG = []; var p = document.getElementById('pp'); p.addEventListener('click', function () { log.push('l'); }); p.dispatchEvent(ev('click')); return log.join(); });
t('unknownAttr', function () { var el = document.createElement('div'); el.setAttribute('onfoo', 'window.FOO=1'); el.dispatchEvent(ev('foo')); return String(window.FOO) + typeof el.onfoo; });
t('caseAttr', function () { var el = document.createElement('div'); el.setAttribute('ONCLICK', 'window.UC=1'); el.dispatchEvent(ev('click')); return String(window.UC) + typeof el.onclick; });
t('detachedAttr', function () { var el = document.createElement('div'); el.setAttribute('onclick', 'window.DA = typeof document + typeof form'); el.dispatchEvent(ev('click')); return window.DA; });
t('withVar', function () { var el = document.createElement('div'); el.id = 'wv'; el.setAttribute('onclick', 'var zz = 1; window.WV = [typeof zz, id, typeof nodeName, typeof URL]'); el.dispatchEvent(ev('click')); return window.WV.join() + typeof window.zz; });
t('thisArg', function () { var el = document.createElement('div'); var th; el.onclick = function () { th = this; }; el.dispatchEvent(ev('click')); return th === el; });
t('handlerName', function () { var el = document.createElement('div'); el.setAttribute('onload', '1'); return el.onload.name + ',' + Object.getOwnPropertyNames(el.onload).join('/'); });
t('xhrOrder', function () { var x = new XMLHttpRequest(); var log = []; x.addEventListener('foo', function () { log.push('l'); }); x.onfoo = 1; x.onload = function () { log.push('h'); }; x.addEventListener('load', function () { log.push('l2'); }); x.dispatchEvent(ev('load')); return log.join(); });
t('abortOrder', function () { var ac = new AbortController(); var log = []; ac.signal.addEventListener('abort', function () { log.push('l1'); }); ac.signal.onabort = function () { log.push('h'); }; ac.abort(); return log.join(); });
t('bodyRemove', function () { var b = document.body, f = function () {}; var r = []; b.setAttribute('onload', 'return 1'); r.push(typeof window.onload); b.removeAttribute('onload'); r.push(typeof window.onload); window.onload = f; b.removeAttribute('onload'); r.push(window.onload === f); b.setAttribute('onload', 'return 2'); r.push(window.onload === f, String(window.onload).length); b.removeAttribute('onload'); r.push(typeof window.onload); window.onload = null; b.setAttribute('onclick', 'return 3'); b.removeAttribute('onclick'); r.push(String(b.onclick)); var nb = document.createElement('body'); nb.setAttribute('onload', 'return 4'); r.push(String(window.onload).length); nb.removeAttribute('onload'); r.push(String(window.onload).length); window.onload = null; return r.join(); });
t('elRemoveAfterIdl', function () { var el = document.createElement('div'); var f = function () {}; el.onclick = f; el.removeAttribute('onclick'); var r = [el.onclick === f]; el.setAttribute('onclick', '1'); el.removeAttribute('onclick'); r.push(el.onclick); return r.join(); });
t('setAttrSame', function () { var log = window.LS = []; var el = document.createElement('div'); el.setAttribute('onclick', "LS.push('a')"); el.addEventListener('click', function () { log.push('l'); }); el.setAttribute('onclick', "LS.push('b')"); el.dispatchEvent(ev('click')); el.onclick = null; el.setAttribute('onclick', "LS.push('c')"); el.dispatchEvent(ev('click')); return log.join(); });
t('cloned', function () { var log = window.LC = []; var el = document.createElement('div'); el.setAttribute('onclick', "LC.push('a' + (this === c))"); var c = el.cloneNode(); window.CC = c; c.setAttribute('onclick', "LC.push('c' + (this === CC))"); c.dispatchEvent(ev('click')); el.dispatchEvent(ev('click')); return log.join(); });
t('imgAttr', function () { return window.IMGLOG.join(' '); });
resolve(out);
