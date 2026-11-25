// slotchange (js.md §15.8): run on Firefox 153.0esr (identity.md §3.19 rig)
// over slotchange/page.html, its answers kept in firefox153.json. Each step
// mutates, then reads the log a task later.
var out = {};
var $ = function (id) { return document.getElementById(id); };
var nm = function (n) { return n === null ? 'null' : n === window ? 'W' : n === document ? 'D' : n instanceof ShadowRoot ? 'SR:' + nm(n.host) : n.id || n.localName || n.nodeName; };
var log = [];
var h = $('h');
var sr = h.attachShadow({ mode: 'open' });
sr.innerHTML = '<slot id=one name=one>f1</slot><slot id=def>fd</slot><slot id=two name=two></slot>';
var hear = function (n) { n.addEventListener('slotchange', function (e) { log.push(nm(e.currentTarget) + '>' + nm(e.target) + ':' + e.bubbles + e.composed + e.cancelable + e.eventPhase + ':' + e.target.assignedNodes().map(nm).join(',')); }); };
hear(sr); hear(h); hear(document); ['one', 'def', 'two'].forEach(function (s) { hear(sr.getElementById(s)); });
hear($('d').shadowRoot);
new MutationObserver(function () { log.push('MO'); }).observe(h, { childList: true, attributes: true });
var steps = [
  ['initial', function () {}],
  ['append', function () { h.appendChild(document.createElement('em')).id = 'c'; }],
  ['text', function () { h.appendChild(document.createTextNode('t')); }],
  ['rename', function () { $('a').slot = 'two'; }],
  ['same', function () { $('a').slot = 'two'; }],
  ['remove', function () { $('b').remove(); }],
  ['slotName', function () { sr.getElementById('two').name = 'zzz'; }],
  ['newSlot', function () { var s = document.createElement('slot'); s.id = 'first'; hear(s); sr.insertBefore(s, sr.firstChild); }],
  ['removeSlot', function () { sr.getElementById('first').remove(); }],
  ['fallback', function () { sr.getElementById('one').textContent = 'changed'; }],
  ['twice', function () { var x = document.createElement('u'); h.appendChild(x); x.remove(); }],
  ['twoChanges', function () { h.appendChild(document.createElement('q')); h.appendChild(document.createElement('q')); }],
  ['declared', function () { $('d').appendChild(document.createElement('s')); }],
  ['nested', function () {
    var n = $('n'); var r = n.attachShadow({ mode: 'open' });
    r.innerHTML = '<div id=inner><slot id=outer></slot></div>';
    var inner = r.getElementById('inner'); var r2 = inner.attachShadow({ mode: 'open' });
    r2.innerHTML = '<slot id=deep></slot>';
    hear(r.getElementById('outer')); hear(r2.getElementById('deep')); hear(r2); hear(r);
  }],
  ['nestedAppend', function () { $('n').appendChild(document.createElement('kbd')); }],
  ['detachedHost', function () { var x = document.createElement('div'); var r = x.attachShadow({ mode: 'open' }); r.innerHTML = '<slot id=ds2></slot>'; hear(r.firstChild); x.appendChild(document.createElement('p')); }],
  ['onslot', function () { var seen = 0; sr.onslotchange = function () { seen++; log.push('handler'); }; h.appendChild(document.createElement('var')); }],
];
var i = 0;
var next = function () {
  if (i > 0) { out[steps[i - 1][0]] = log.join(' | '); }
  if (i === steps.length) { resolve(out); return; }
  log.length = 0;
  try { steps[i][1](); } catch (e) { log.push('THROW ' + e.name + ': ' + e.message); }
  log.push('sync');
  i++;
  setTimeout(next, 0);
};
next();
