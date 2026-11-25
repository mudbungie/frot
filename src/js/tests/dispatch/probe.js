// Event dispatch probe (bl-810b): run on Firefox 153.0esr (identity.md §3.19 rig)
// over dispatch/page.html, its answers kept verbatim in firefox153.json.
var out = {};
var t = function (k, f) { try { out[k] = f(); } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var host = document.createElement('div');
host.id = 'host';
host.innerHTML = '<div id=a><p id=b><i id=c>x</i></p></div>';
document.body.appendChild(host);
var A = document.getElementById('a'), B = document.getElementById('b'), C = document.getElementById('c');
function nm(o) { return o === null ? 'null' : o === undefined ? 'undef' : o === window ? 'win' : o === document ? 'doc' : o === document.documentElement ? 'html' : o === document.body ? 'body' : o.nodeType === 11 ? 'frag' : o.nodeType === 3 ? '#t' : o.id || o.nodeName || String(o); }
var NODES = [window, document, document.documentElement, document.body, host, A, B, C];
// Listen on every node at both phases; record "name:phase:eventPhase".
function rig(type, opts) {
  var log = [], offs = [];
  NODES.forEach(function (n) {
    [true, false].forEach(function (cap) {
      var f = function (e) {
        log.push(nm(this) + (cap ? 'C' : 'B') + e.eventPhase + (e.currentTarget === this ? '' : '!') + (opts && opts.each ? opts.each(e, this, cap) : ''));
      };
      n.addEventListener(type, f, cap);
      offs.push(function () { n.removeEventListener(type, f, cap); });
    });
  });
  return { log: log, off: function () { offs.forEach(function (o) { o(); }); } };
}
function run(type, init, target, opts) {
  var r = rig(type, opts);
  var ev = new Event(type, init);
  var ret = (target || C).dispatchEvent(ev);
  r.off();
  return r.log.join(' ') + '|' + ret + '|' + ev.eventPhase + nm(ev.currentTarget) + nm(ev.target) + ev.composedPath().length;
}
t('evProto', function () { return Object.getOwnPropertyNames(Event.prototype).join(); });
t('evStatic', function () { return Object.getOwnPropertyNames(Event).join(); });
t('etProto', function () { return Object.getOwnPropertyNames(EventTarget.prototype).join(); });
t('etLens', function () { return ['addEventListener', 'removeEventListener', 'dispatchEvent'].map(function (k) { return EventTarget.prototype[k].length; }).join(); });
t('evOwn', function () { return Object.getOwnPropertyNames(new Event('x')).join(); });
t('fresh', function () { var e = new Event('x'); return [e.type, e.target, e.currentTarget, e.srcElement, e.eventPhase, e.bubbles, e.cancelable, e.composed, e.defaultPrevented, e.returnValue, e.cancelBubble, e.isTrusted, typeof e.timeStamp, e.composedPath().length].join(); });
t('init', function () { var e = new Event('x', { bubbles: 1, cancelable: 'y', composed: {} }); return [e.bubbles, e.cancelable, e.composed].join(); });
t('noBubble', function () { return run('nb'); });
t('bubble', function () { return run('bb', { bubbles: true }); });
t('atHost', function () { return run('h', { bubbles: true }, host); });
t('atDoc', function () { return run('d', { bubbles: true }, document); });
t('atWin', function () { return run('w', { bubbles: true }, window); });
t('load', function () { return run('load', { bubbles: true }); });
t('error', function () { return run('error', {}); });
t('loadDoc', function () { return run('load', {}, document); });
t('path', function () { var p; var f = function (e) { p = e.composedPath().map(nm).join(); }; C.addEventListener('pp', f); C.dispatchEvent(new Event('pp')); C.removeEventListener('pp', f); return p; });
t('pathLoad', function () { var p; var f = function (e) { p = e.composedPath().map(nm).join(); }; C.addEventListener('load', f); C.dispatchEvent(new Event('load')); C.removeEventListener('load', f); return p; });
// stopping
['winC', 'docC', 'bC', 'cC', 'cB', 'bB', 'docB'].forEach(function (where) {
  t('stop ' + where, function () { return run('s', { bubbles: true }, C, { each: function (e, th, cap) { if (nm(th) + (cap ? 'C' : 'B') === where.slice(0, -1) + where.slice(-1)) e.stopPropagation(); return ''; } }); });
});
t('stopImm', function () {
  var log = [];
  B.addEventListener('si', function () { log.push(1); });
  B.addEventListener('si', function (e) { log.push(2); e.stopImmediatePropagation(); });
  B.addEventListener('si', function () { log.push(3); });
  A.addEventListener('si', function () { log.push(4); });
  C.dispatchEvent(new Event('si', { bubbles: true }));
  return log.join();
});
t('stopSame', function () {
  var log = [];
  B.addEventListener('ss', function (e) { log.push(1); e.stopPropagation(); }, true);
  B.addEventListener('ss', function () { log.push(2); }, true);
  B.addEventListener('ss', function () { log.push(3); });
  C.addEventListener('ss', function () { log.push(4); });
  C.dispatchEvent(new Event('ss', { bubbles: true }));
  B.dispatchEvent(new Event('ss', { bubbles: true }));
  return log.join();
});
t('cancelBubble', function () {
  var log = [], e0 = new Event('cb', { bubbles: true });
  e0.cancelBubble = true;
  var s1 = e0.cancelBubble;
  C.addEventListener('cb', function () { log.push('c'); });
  C.dispatchEvent(e0);
  var e = new Event('cb', { bubbles: true });
  B.addEventListener('cb', function (ev) { log.push('b' + ev.cancelBubble); ev.cancelBubble = true; log.push(ev.cancelBubble); ev.cancelBubble = false; log.push(ev.cancelBubble); });
  A.addEventListener('cb', function () { log.push('a'); });
  C.dispatchEvent(e);
  return s1 + ';' + log.join() + ';' + e.cancelBubble;
});
t('stopBefore', function () { var e = new Event('sb', { bubbles: true }); e.stopPropagation(); var n = 0; C.addEventListener('sb', function () { n++; }); C.dispatchEvent(e); C.dispatchEvent(e); return n + ',' + e.cancelBubble; });
// cancel
t('cancel', function () {
  var r = [];
  [{}, { cancelable: true }].forEach(function (init) {
    var e = new Event('cx', init);
    var f = function (ev) { ev.preventDefault(); r.push(ev.defaultPrevented); };
    C.addEventListener('cx', f);
    r.push(C.dispatchEvent(e), e.defaultPrevented, e.returnValue);
    C.removeEventListener('cx', f);
  });
  var e2 = new Event('rv', { cancelable: true }); e2.returnValue = false; r.push(e2.defaultPrevented); e2.returnValue = true; r.push(e2.defaultPrevented);
  var e3 = new Event('pd', { cancelable: true }); e3.preventDefault(); r.push(e3.defaultPrevented, C.dispatchEvent(e3), e3.defaultPrevented);
  return r.join();
});
t('passive', function () {
  var r = [];
  var e = new Event('pv', { cancelable: true });
  var f = function (ev) { ev.preventDefault(); r.push(ev.defaultPrevented); };
  C.addEventListener('pv', f, { passive: true });
  r.push(C.dispatchEvent(e), e.defaultPrevented);
  C.removeEventListener('pv', f);
  return r.join();
});
// options
t('once', function () { var n = 0; var f = function () { n++; }; C.addEventListener('o', f, { once: true }); C.dispatchEvent(new Event('o')); C.dispatchEvent(new Event('o')); return n; });
t('onceReenter', function () { var n = 0; var f = function () { n++; if (n < 3) C.dispatchEvent(new Event('or')); }; C.addEventListener('or', f, { once: true }); C.dispatchEvent(new Event('or')); return n; });
t('dedupe', function () {
  var n = [];
  var f = function (e) { n.push(e.eventPhase); };
  A.addEventListener('dd', f); A.addEventListener('dd', f); A.addEventListener('dd', f, true); A.addEventListener('dd', f, { capture: true, once: true }); A.addEventListener('dd', f, { capture: false, passive: true });
  C.dispatchEvent(new Event('dd', { bubbles: true }));
  A.removeEventListener('dd', f, { capture: true });
  C.dispatchEvent(new Event('dd', { bubbles: true }));
  A.removeEventListener('dd', f);
  C.dispatchEvent(new Event('dd', { bubbles: true }));
  return n.join();
});
t('optForms', function () {
  var n = [];
  [1, 0, 'x', '', null, undefined, {}, { capture: 1 }, [], function () {}].forEach(function (o, i) {
    var f = function (e) { n.push(i + ':' + e.eventPhase); };
    A.addEventListener('of' + i, f, o);
    C.dispatchEvent(new Event('of' + i, { bubbles: true }));
  });
  return n.join();
});
t('atTargetOrder', function () {
  var log = [];
  C.addEventListener('ato', function () { log.push('b1'); });
  C.addEventListener('ato', function () { log.push('c1'); }, true);
  C.addEventListener('ato', function () { log.push('b2'); });
  C.addEventListener('ato', function () { log.push('c2'); }, true);
  C.dispatchEvent(new Event('ato'));
  return log.join();
});
t('handleEvent', function () {
  var log = [];
  var o = { handleEvent: function (e) { log.push(this === o, e.type); } };
  C.addEventListener('he', o);
  C.dispatchEvent(new Event('he'));
  o.handleEvent = function () { log.push('swapped'); };
  C.dispatchEvent(new Event('he'));
  var fo = function () { log.push('fn'); }; fo.handleEvent = function () { log.push('he'); };
  C.addEventListener('he2', fo);
  C.dispatchEvent(new Event('he2'));
  var bad = {};
  C.addEventListener('he3', bad);
  C.addEventListener('he3', function () { log.push('after'); });
  var errs = [];
  var onerr = function (e) { errs.push(e.constructor.name + ':' + e.message); e.preventDefault(); };
  window.addEventListener('error', onerr);
  C.dispatchEvent(new Event('he3'));
  window.removeEventListener('error', onerr);
  return log.join() + ';' + errs.join();
});
t('throwing', function () {
  var log = [], errs = [];
  var onerr = function (e) { errs.push([e.constructor.name, e.message, e.error && e.error.message, e.filename === location.href || e.filename, e.cancelable, e.isTrusted].join('/')); e.preventDefault(); };
  window.addEventListener('error', onerr);
  C.addEventListener('th', function () { log.push(1); throw new Error('boom'); });
  C.addEventListener('th', function () { log.push(2); });
  var ret = C.dispatchEvent(new Event('th', { bubbles: true }));
  window.removeEventListener('error', onerr);
  return log.join() + ';' + ret + ';' + errs.join();
});
t('nullfn', function () { C.addEventListener('nf', null); C.addEventListener('nf', undefined); C.removeEventListener('nf', null); return C.dispatchEvent(new Event('nf')); });
t('badfn', function () { C.addEventListener('bf', 5); });
t('badfn2', function () { C.addEventListener('bf', 'str'); });
t('noargs', function () { C.addEventListener('x'); });
t('noargs0', function () { C.addEventListener(); });
t('dispNoArg', function () { C.dispatchEvent(); });
t('dispNotEv', function () { C.dispatchEvent({ type: 'x' }); });
t('dispNull', function () { C.dispatchEvent(null); });
t('reentrant', function () { var e = new Event('re'); var r; C.addEventListener('re', function () { try { C.dispatchEvent(e); r = 'ok'; } catch (x) { r = x.name + ': ' + x.message; } }); C.dispatchEvent(e); return r; });
t('redispatch', function () { var e = new Event('rd', { bubbles: true }); var log = []; var f = function (ev) { log.push(nm(ev.target) + nm(ev.currentTarget)); }; C.addEventListener('rd', f); A.addEventListener('rd', f); C.dispatchEvent(e); var after = nm(e.target); A.dispatchEvent(e); return log.join() + ';' + after + nm(e.target); });
t('removeDuring', function () {
  var log = [];
  var g2 = function () { log.push(2); };
  var g3 = function () { log.push(3); };
  C.addEventListener('rmd', function () { log.push(1); C.removeEventListener('rmd', g2); C.addEventListener('rmd', function () { log.push('new'); }); A.addEventListener('rmd', function () { log.push('newA'); }); });
  C.addEventListener('rmd', g2);
  C.addEventListener('rmd', g3);
  C.dispatchEvent(new Event('rmd', { bubbles: true }));
  return log.join();
});
t('detached', function () { var d = document.createElement('div'); d.innerHTML = '<span></span>'; var s = d.firstChild; var log = []; [d, s, document, window].forEach(function (n) { n.addEventListener('dt', function (e) { log.push(nm(this) + e.eventPhase); }, true); n.addEventListener('dt', function (e) { log.push(nm(this) + e.eventPhase); }); }); s.dispatchEvent(new Event('dt', { bubbles: true })); return log.join() + '|' + d.nodeName; });
t('fragment', function () { var f = document.createDocumentFragment(); var s = f.appendChild(document.createElement('span')); var log = []; [f, s, document, window].forEach(function (n) { n.addEventListener('fr', function (e) { log.push(nm(this) + e.eventPhase); }); }); s.dispatchEvent(new Event('fr', { bubbles: true })); return log.join(); });
t('textNode', function () { var tx = C.firstChild; var log = []; [tx, C, document].forEach(function (n) { n.addEventListener('tn', function (e) { log.push(nm(this) + e.eventPhase + nm(e.target)); }); }); tx.dispatchEvent(new Event('tn', { bubbles: true })); return log.join(); });
t('moveDuring', function () { var log = []; C.addEventListener('mv', function () { log.push('c'); document.body.appendChild(C); }); B.addEventListener('mv', function () { log.push('b'); }); A.addEventListener('mv', function () { log.push('a'); }); C.dispatchEvent(new Event('mv', { bubbles: true })); B.appendChild(C); return log.join(); });
t('custom', function () { var e = new CustomEvent('cu', { detail: { a: 1 }, bubbles: true }); var d; A.addEventListener('cu', function (ev) { d = ev.detail.a + nm(ev.target) + ev.eventPhase; }); C.dispatchEvent(e); return d; });
t('srcElement', function () { var s; C.addEventListener('se', function (e) { s = nm(e.srcElement) + nm(e.target); }); C.dispatchEvent(new Event('se')); return s; });
t('thisPlain', function () { var th; window.addEventListener('tp', function () { th = this === window; }); window.dispatchEvent(new Event('tp')); return th; });
t('bareCall', function () { var n = 0; var add = EventTarget.prototype.addEventListener; add('bc', function () { n++; }); window.dispatchEvent(new Event('bc')); return n; });
t('signal', function () { var ac = new AbortController(); var n = 0; C.addEventListener('sg', function () { n++; }, { signal: ac.signal }); C.dispatchEvent(new Event('sg')); ac.abort(); C.dispatchEvent(new Event('sg')); var ac2 = new AbortController(); ac2.abort(); C.addEventListener('sg2', function () { n += 10; }, { signal: ac2.signal }); C.dispatchEvent(new Event('sg2')); return n; });
t('signalBad', function () { C.addEventListener('sb', function () {}, { signal: 5 }); });
t('initEvent', function () { var e = document.createEvent('Event'); var r = [e.type, e.bubbles]; e.initEvent('ie', true, true); r.push(e.type, e.bubbles, e.cancelable); var n = 0; A.addEventListener('ie', function () { n++; }); C.dispatchEvent(e); r.push(n); return r.join(); });
t('uninit', function () { return C.dispatchEvent(document.createEvent('Event')); });
t('newET', function () { var et = new EventTarget(); var log = []; et.addEventListener('z', function (e) { log.push(e.eventPhase, e.target === et, e.composedPath().length); }); et.dispatchEvent(new Event('z', { bubbles: true })); return log.join(); });
t('phaseConsts', function () { return [Event.NONE, Event.CAPTURING_PHASE, Event.AT_TARGET, Event.BUBBLING_PHASE, Event.prototype.AT_TARGET].join(); });
t('timeStamp', function () { var e = new Event('x'); return typeof e.timeStamp + (e.timeStamp > 0) + (e.timeStamp <= performance.now()); });
resolve(out);
