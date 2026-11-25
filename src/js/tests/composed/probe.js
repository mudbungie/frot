// Events across shadow boundaries (js.md §15.7): run on Firefox 153.0esr
// (identity.md §3.19 rig) over composed/page.html, its answers kept in
// firefox153.json.
var out = {};
var t = function (k, f) { try { var v = f(); out[k] = v === undefined ? 'undefined' : v; } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var $ = function (id) { return document.getElementById(id); };
var nm = function (n) { return n === null ? 'null' : n === undefined ? 'undef' : n === window ? 'W' : n === document ? 'D' : n instanceof ShadowRoot ? 'SR:' + nm(n.host) : n.id || n.localName || n.nodeName; };
var h = $('h');
var sr = h.attachShadow({ mode: 'open' });
sr.innerHTML = '<div id=in><b id=deep>d</b><y-in id=inner></y-in></div><slot name=s id=sl></slot>';
var inner = sr.getElementById('inner');
var sr2 = inner.attachShadow({ mode: 'open' });
sr2.innerHTML = '<em id=deeper>e</em>';
var ch = $('ch');
var csr = ch.attachShadow({ mode: 'closed' });
csr.innerHTML = '<p id=cin><a id=clink>c</a></p>';
var log = [];
var watch = function (list) {
  list.forEach(function (n) {
    [true, false].forEach(function (cap) {
      n.addEventListener('x', function (e) {
        log.push(nm(e.currentTarget) + (cap ? '^' : 'v') + e.eventPhase + '>' + nm(e.target) + '[' + e.composedPath().map(nm).join(',') + ']');
      }, cap);
    });
  });
};
watch([window, document, $('outer'), h, sr, sr.getElementById('in'), sr.getElementById('deep'), sr.getElementById('sl'), $('light'), inner, sr2, sr2.getElementById('deeper'), ch, csr, csr.getElementById('cin')]);
var fire = function (node, init) { log.length = 0; var e = new Event('x', init); var r = node.dispatchEvent(e); log.push('after>' + nm(e.target) + '/' + nm(e.currentTarget) + '/' + e.composedPath().length + '/' + e.eventPhase + '/' + r); return log.join(' '); };
t('composedBubbles', function () { return fire(sr.getElementById('deep'), { bubbles: true, composed: true }); });
t('uncomposedBubbles', function () { return fire(sr.getElementById('deep'), { bubbles: true }); });
t('composedOnly', function () { return fire(sr.getElementById('deep'), { composed: true }); });
t('plain', function () { return fire(sr.getElementById('deep'), {}); });
t('slotted', function () { return fire($('light'), { bubbles: true }); });
t('slottedComposed', function () { return fire($('light'), { bubbles: true, composed: true }); });
t('nested', function () { return fire(sr2.getElementById('deeper'), { bubbles: true, composed: true }); });
t('nestedUncomposed', function () { return fire(sr2.getElementById('deeper'), { bubbles: true }); });
t('atRoot', function () { return fire(sr, { bubbles: true, composed: true }); });
t('atRootUncomposed', function () { return fire(sr, { bubbles: true }); });
t('closed', function () { return fire(csr.getElementById('clink'), { bubbles: true, composed: true }); });
t('closedRoot', function () { return fire(csr.getElementById('cin'), { bubbles: true, composed: false }); });
t('stray', function () { return fire($('stray'), { bubbles: true, composed: true }); });
t('detached', function () {
  var d = document.createElement('div'); var r = d.attachShadow({ mode: 'open' }); r.innerHTML = '<b id=q>q</b>';
  var seen = []; d.addEventListener('x', function (e) { seen.push(nm(e.target) + ':' + e.eventPhase); });
  r.firstChild.addEventListener('x', function (e) { seen.push(e.composedPath().map(nm).join(',')); });
  var e = new Event('x', { bubbles: true, composed: true }); r.firstChild.dispatchEvent(e);
  return seen.join(' ') + ' ' + nm(e.target);
});
t('stop', function () {
  var f = function (e) { e.stopPropagation(); };
  sr.addEventListener('x', f);
  var r = fire(sr.getElementById('deep'), { bubbles: true, composed: true });
  sr.removeEventListener('x', f);
  return r;
});
t('retargetHandler', function () {
  var seen = [];
  h.onclick = function (e) { seen.push(nm(e.target)); };
  sr.getElementById('deep').dispatchEvent(new Event('click', { bubbles: true, composed: true }));
  h.onclick = null;
  return seen.join(',');
});
resolve(out);
