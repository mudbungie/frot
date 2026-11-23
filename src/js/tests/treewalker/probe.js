// TreeWalker probe (bl-b8c0): run on Firefox 153.0esr (identity.md §3.19 rig) over
// tests/../treewalker/page.html, its answers kept verbatim in firefox153.json.
var out = {};
var t = function (k, f) { try { out[k] = f(); } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var host = document.createElement('div');
host.innerHTML = '<a id=a>t1<b id=b><!--c1--><i id=i>t2</i></b>t3<u id=u></u></a><s id=s>t4<!--c2--></s><q id=q><r id=r>t5</r></q>';
document.body.appendChild(host);
function nm(n) { return n === null ? 'null' : n.nodeType === 1 ? n.id || n.tagName : n.nodeType === 3 ? n.data : n.nodeType === 8 ? '!' + n.data : n.nodeName; }
var W = [-1, 1, 4, 128, 5, 133, 0];
var F = {
  none: null,
  skipB: function (n) { return n.id === 'b' ? 3 : 1; },
  rejB: function (n) { return n.id === 'b' ? 2 : 1; },
  rejQ: function (n) { return n.id === 'q' || n.id === 'a' ? 2 : 1; },
  skipAll: function (n) { return n.nodeType === 1 ? 3 : 1; },
  str: function (n) { return n.id === 's' ? '2' : 1.9; },
  obj: { acceptNode: function (n) { return n.nodeType === 3 ? 3 : 1; } },
};
function walk(w, op, max) { var r = []; for (var i = 0; i < (max || 40); i++) { var n = w[op](); if (!n) break; r.push(nm(n)); } return r.join(' ') + '|' + nm(w.currentNode); }
Object.keys(F).forEach(function (fk) { W.forEach(function (ws) {
  var k = fk + ':' + ws;
  t(k + ' next', function () { return walk(document.createTreeWalker(host, ws, F[fk]), 'nextNode'); });
  t(k + ' prev', function () { var w = document.createTreeWalker(host, ws, F[fk]); w.currentNode = document.getElementById('r').firstChild; return walk(w, 'previousNode'); });
  t(k + ' prevEnd', function () { var w = document.createTreeWalker(host, ws, F[fk]); while (w.nextNode()); return walk(w, 'previousNode'); });
  t(k + ' kids', function () { var w = document.createTreeWalker(host, ws, F[fk]); var r = []; var n = w.firstChild(); while (n) { r.push(nm(n)); n = w.nextSibling(); } r.push('/'); n = w.parentNode(); r.push(nm(n)); n = w.lastChild(); while (n) { r.push(nm(n)); n = w.previousSibling(); } return r.join(' '); });
  t(k + ' deep', function () { var w = document.createTreeWalker(host, ws, F[fk]); w.currentNode = document.getElementById('i').firstChild; var r = []; var n; while ((n = w.parentNode())) r.push(nm(n)); w.currentNode = document.getElementById('i'); r.push('/', nm(w.nextSibling()), nm(w.previousSibling()), nm(w.firstChild()), nm(w.lastChild())); w.currentNode = document.getElementById('b'); r.push(nm(w.nextSibling()), nm(w.nextSibling()), nm(w.previousSibling())); return r.join(' '); });
}); });
t('outside', function () { var w = document.createTreeWalker(document.getElementById('b')); w.currentNode = document.getElementById('s'); return walk(w, 'nextNode') + ';' + (w.currentNode = document.getElementById('s'), walk(w, 'previousNode')) + ';' + (w.currentNode = document.getElementById('r'), nm(w.parentNode()) + nm(w.nextSibling()) + nm(w.previousSibling())); });
t('rootsib', function () { var w = document.createTreeWalker(document.getElementById('b')); return [nm(w.nextSibling()), nm(w.previousSibling()), nm(w.parentNode()), nm(w.previousNode())].join(); });
t('this', function () { var th = []; var f = function (n) { th.push(this === undefined ? 'u' : this === f ? 'f' : this === globalThis ? 'g' : typeof this); return 1; }; document.createTreeWalker(host, 1, f).nextNode(); var o = { acceptNode: function () { th.push(this === o ? 'o' : '?'); return 1; } }; document.createTreeWalker(host, 1, o).nextNode(); return th.join(); });
t('fnWithAccept', function () { var f = function () { return 2; }; f.acceptNode = function () { return 1; }; return nm(document.createTreeWalker(host, 1, f).nextNode()); });
t('noAccept', function () { return document.createTreeWalker(host, 1, {}).nextNode(); });
t('badAccept', function () { return document.createTreeWalker(host, 1, { acceptNode: 5 }).nextNode(); });
t('throws', function () { return document.createTreeWalker(host, 1, function () { throw new RangeError('boom'); }).nextNode(); });
t('reenter', function () { var w; w = document.createTreeWalker(host, 1, function (n) { w.nextNode(); return 1; }); return w.nextNode(); });
t('reenter2', function () { var w; w = document.createTreeWalker(host, 1, function (n) { return w.firstChild() ? 1 : 1; }); return w.firstChild(); });
t('reenterOther', function () { var w2 = document.createTreeWalker(host, 1); var w; w = document.createTreeWalker(host, 1, function (n) { w2.nextNode(); return 1; }); return nm(w.nextNode()); });
t('ret', function () { return [undefined, null, '', 'x', 65537, -1, {}, true, 3.9, 2**16+3].map(function (v) { return nm(document.createTreeWalker(host, 1, function (n) { return n.id === 'a' ? v : 2; }).nextNode()); }).join(); });
t('retSym', function () { return document.createTreeWalker(host, 1, function () { return Symbol(); }).nextNode(); });
t('setcur', function () { var w = document.createTreeWalker(host); w.currentNode = {}; });
t('setcurnull', function () { var w = document.createTreeWalker(host); w.currentNode = null; });
t('setcurdoc', function () { var w = document.createTreeWalker(host); w.currentNode = document; return nm(w.currentNode) + nm(w.nextNode()); });
t('wtsHigh', function () { return walk(document.createTreeWalker(document, 256 | 512), 'nextNode'); });
t('docwalk', function () { return walk(document.createTreeWalker(document, 1), 'nextNode', 5); });
t('recv', function () { return TreeWalker.prototype.nextNode.call({}); });
t('recvget', function () { return Object.getOwnPropertyDescriptor(TreeWalker.prototype, 'root').get.call({}); });
t('mut', function () { var w = document.createTreeWalker(host, 1); var r = []; var n; while ((n = w.nextNode())) { r.push(nm(n)); if (n.id === 'b') { var bb = n; n.remove(); } } host.querySelector('a').insertBefore(bb, document.getElementById('u')); return r.join(' '); });
t('litlike', function () { var tpl = document.createElement('template'); tpl.innerHTML = '<div>x<!--?lit$1$--><p>y</p></div>'; var c = tpl.content; var w = document.createTreeWalker(document, 129); w.currentNode = c; var r = []; var n; while ((n = w.nextNode())) r.push(nm(n)); return r.join(' ') + '|' + (w.root === document); });
t('frag', function () { var f = document.createDocumentFragment(); f.appendChild(document.createElement('p')).id = 'p1'; f.appendChild(document.createComment('z')); return walk(document.createTreeWalker(f, 1029), 'nextNode'); });
t('newTW', function () { return new TreeWalker(); });
t('newNI', function () { return new NodeIterator(); });
t('newNF', function () { return new NodeFilter(); });
t('NFcall', function () { return NodeFilter(); });
t('ctw', function () { var w = document.createTreeWalker(document.body); return [Object.prototype.toString.call(w), w.root === document.body, w.whatToShow, w.filter, w.currentNode === document.body, Object.getOwnPropertyNames(w).length]; });
t('ctwlen', function () { return [document.createTreeWalker.length, document.createNodeIterator.length]; });
t('ctw0', function () { return document.createTreeWalker(); });
t('ctwbad', function () { return document.createTreeWalker({}); });
t('ctwnull', function () { return document.createTreeWalker(null); });
t('ctwwts', function () { return [document.createTreeWalker(document, -1).whatToShow, document.createTreeWalker(document, 2**32+5).whatToShow, document.createTreeWalker(document, '129').whatToShow, document.createTreeWalker(document, undefined).whatToShow, document.createTreeWalker(document, null).whatToShow]; });
t('ctwfilt', function () { var f = function(){return 1}; var o = {acceptNode: f}; return [document.createTreeWalker(document, 1, f).filter === f, document.createTreeWalker(document, 1, o).filter === o, document.createTreeWalker(document, 1, null).filter]; });
t('ctwfiltbad', function () { return document.createTreeWalker(document, 1, 5); });
t('ctwfiltstr', function () { return document.createTreeWalker(document, 1, 'x'); });
t('NF', function () { return Object.getOwnPropertyNames(NodeFilter).map(function (k) { return k + '=' + String(NodeFilter[k]); }).join(' ') + ' ' + ('prototype' in NodeFilter); });
t('TWproto', function () { return Object.getOwnPropertyNames(TreeWalker.prototype).join(' '); });
t('TWlen', function () { return ['parentNode', 'nextNode', 'createTreeWalker'].map(function (k) { return (TreeWalker.prototype[k] || document[k]).length; }); });
resolve(out);
