// Manual slot assignment (js.md §15.9): run on Firefox 153.0esr (identity.md
// §3.19 rig) over assign/page.html, its answers kept in firefox153.json.
var out = {};
var $ = function (id) { return document.getElementById(id); };
var nm = function (n) { return n === null ? 'null' : n === undefined ? 'undef' : n.nodeType === 3 ? JSON.stringify(n.data) : n instanceof ShadowRoot ? 'SR' : n.id || n.localName; };
var t = function (k, f) { try { var v = f(); out[k] = v === undefined ? 'undefined' : v; } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var log = [];
var h = $('h');
var sr = h.attachShadow({ mode: 'open', slotAssignment: 'manual' });
sr.innerHTML = '<slot id=one name=one>f1</slot><slot id=two>f2</slot>';
var one = sr.getElementById('one'), two = sr.getElementById('two');
[one, two].forEach(function (s) { s.addEventListener('slotchange', function (e) { log.push(nm(e.target) + ':' + e.target.assignedNodes().map(nm).join(',')); }); });
var A = function (s) { return s.assignedNodes().map(nm).join(',') + '|' + s.assignedNodes({ flatten: true }).map(nm).join(','); };
var text = h.childNodes[2];
var steps = [
  ['proto', function () { return typeof HTMLSlotElement.prototype.assign + ' ' + HTMLSlotElement.prototype.assign.length + ' ' + sr.slotAssignment; }],
  ['initial', function () { return A(one) + ' ' + A(two) + ' ' + nm($('a').assignedSlot); }],
  ['assign', function () { return String(one.assign($('b'), $('a'))) + ' ' + A(one) + ' ' + nm($('a').assignedSlot) + ' ' + nm($('b').assignedSlot); }],
  ['text', function () { two.assign(text, $('c')); return A(two) + ' ' + nm(text.assignedSlot); }],
  ['steal', function () { two.assign($('a')); return A(one) + ' ' + A(two); }],
  ['dup', function () { one.assign($('b'), $('b')); return A(one); }],
  ['notChild', function () { one.assign($('o'), $('b')); return A(one) + ' ' + nm($('o').assignedSlot); }],
  ['bad', function () { one.assign(document.createComment('x')); }],
  ['badStr', function () { one.assign('x'); }],
  ['moveIn', function () { h.appendChild($('o')); return A(one) + ' ' + nm($('o').assignedSlot); }],
  ['moveOut', function () { $('other').appendChild($('o')); return A(one); }],
  ['moveBack', function () { h.appendChild($('o')); return A(one); }],
  ['clear', function () { one.assign(); return A(one) + ' ' + nm($('b').assignedSlot); }],
  ['slotMoved', function () { two.assign($('c')); sr.insertBefore(two, one); return A(two); }],
  ['slotOut', function () { two.remove(); var a = A(two) + ' ' + nm($('c').assignedSlot); sr.appendChild(two); return a + ' ' + A(two); }],
  ['named', function () { var r = $('n').attachShadow({ mode: 'open' }); r.innerHTML = '<slot name=x id=nx></slot><slot id=ny></slot>'; var ny = r.getElementById('ny'); ny.assign($('e')); return A(r.getElementById('nx')) + ' ' + A(ny); }],
  ['declared', function () { var r = $('d').shadowRoot; return r.slotAssignment + ' ' + A(r.getElementById('ds')); }],
  ['declaredAssign', function () { var s = $('d').shadowRoot.getElementById('ds'); s.assign($('du')); return A(s); }],
  ['detachedSlot', function () { var s = document.createElement('slot'); s.assign($('b')); return A(s) + ' ' + nm($('b').assignedSlot); }],
  ['otherHost', function () { var x = document.createElement('div'); var r = x.attachShadow({ mode: 'open', slotAssignment: 'manual' }); r.innerHTML = '<slot></slot>'; r.firstChild.assign($('c')); return A(r.firstChild) + ' ' + A(two) + ' ' + nm($('c').assignedSlot); }],
  ['textContent', function () { return h.shadowRoot.innerHTML + ' ' + Array.prototype.map.call(h.childNodes, nm).join(','); }],
];
var i = 0;
var next = function () {
  if (i > 0) { out[steps[i - 1][0] + 'Events'] = log.join(' '); }
  if (i === steps.length) { resolve(out); return; }
  log.length = 0;
  t(steps[i][0], steps[i][1]);
  i++;
  setTimeout(next, 0);
};
next();
