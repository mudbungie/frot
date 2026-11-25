// Frame lifecycle probe (bl-2aaa): window[i], removal closing a window, and
// cross-document adoption; run on Firefox 153.0esr (identity.md §3.19 rig)
// over framelife/page.html, answers in firefox153.json.
var out = {};
var t = function (k, f) { try { out[k] = f(); } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
function dsc(o, k) { var x = Object.getOwnPropertyDescriptor(o, k); if (!x) return 'none'; return (x.get ? 'g' : '') + (x.set ? 's' : '') + ('value' in x ? 'v' : '') + (x.enumerable ? 'e' : '') + (x.configurable ? 'c' : '') + (x.writable ? 'w' : ''); }
var host = document.createElement('div'); document.body.appendChild(host);
var f1 = document.createElement('iframe'), f2 = document.createElement('iframe');
t('before', function () { return [window.length, String(window[0]), dsc(window, '0'), 0 in window].join(); });
host.appendChild(f1); host.appendChild(f2);
var W1 = f1.contentWindow, W2 = f2.contentWindow;
t('indexed', function () { return [window.length, window[0] === W1, window[1] === W2, String(window[2]), dsc(window, '0'), 0 in window, 2 in window, window.frames[1] === W2, Object.keys(window).slice(0, 3).join('/'), Object.getOwnPropertyNames(window).indexOf('0'), Object.getOwnPropertyNames(window).indexOf('1')].join(); });
t('setIdx', function () { window[0] = 5; return window[0] === W1; });
t('setIdxStrict', function () { 'use strict'; window[0] = 5; return window[0] === W1; });
t('defIdx', function () { Object.defineProperty(window, '0', { value: 1 }); return window[0] === W1; });
t('lazyIdx', function () { var f3 = document.createElement('iframe'); host.appendChild(f3); var r = [window.length, window[2] === f3.contentWindow, typeof window[2]]; f3.remove(); return r.join(); });
t('closedDesc', function () { return [dsc(window, 'closed'), window.closed, W1.closed, dsc(window, 'parent'), dsc(window, 'top'), dsc(window, 'frameElement')].join(); });
t('removed', function () { f1.remove(); return [f1.contentWindow, W1.closed, W1.parent, W1.top, W1.frameElement, W1.eval('1+1'), typeof W1.document, W1.document && W1.document.body ? 'body' : 'nobody', window.length, window[0] === W2, String(window[1]), W1.length].join(); });
t('reinserted', function () { host.appendChild(f1); var N = f1.contentWindow; return [N === W1, N.closed, N.parent === window, W1.closed, window[1] === N].join(); });
t('moved', function () { var W = f2.contentWindow; W.zz = 1; document.body.appendChild(f2); return [f2.contentWindow === W, W.closed, typeof f2.contentWindow.zz].join(); });
// adoption
t('adopt', function () { var FD = f1.contentDocument, FW = f1.contentWindow; var p = FD.createElement('p'); p.id = 'ad'; p.innerHTML = 'x<b>y</b>'; var b = p.firstElementChild; FD.body.appendChild(p); var r = [p.ownerDocument === FD]; host.appendChild(p); r.push(p.ownerDocument === document, p.parentNode === host, document.getElementById('ad') === p, p instanceof HTMLParagraphElement, p instanceof FW.HTMLParagraphElement, Object.getPrototypeOf(p) === HTMLParagraphElement.prototype, b.ownerDocument === document, b.parentNode === p, FD.body.childNodes.length, p.textContent, typeof p.appendChild === 'function' && p.appendChild === host.appendChild); return r.join(); });
t('adoptListener', function () { var FD = f1.contentDocument; var p = FD.createElement('span'); var n = 0; p.addEventListener('zz', function () { n++; }); host.appendChild(p); p.dispatchEvent(new Event('zz')); return n; });
t('adoptText', function () { var FD = f1.contentDocument; var tx = FD.createTextNode('tt'); host.appendChild(tx); return [tx.ownerDocument === document, tx.parentNode === host, tx.data].join(); });
t('adoptFrag', function () { var FD = f1.contentDocument; var fr = FD.createDocumentFragment(); fr.appendChild(FD.createElement('i')).id = 'fi'; host.appendChild(fr); var i = document.getElementById('fi'); return [!!i, i && i.ownerDocument === document, fr.childNodes.length].join(); });
t('adoptNodeApi', function () { var FD = f1.contentDocument; var q = FD.createElement('q'); var r = document.adoptNode(q); return [r === q, q.ownerDocument === document, q.parentNode].join(); });
t('importNodeApi', function () { var FD = f1.contentDocument; var q = FD.createElement('q'); q.id = 'iq'; var r = document.importNode(q, true); return [r === q, r.ownerDocument === document, q.ownerDocument === FD, r.id].join(); });
t('parentAdoptedIntoFrame', function () { var FD = f1.contentDocument; var s = document.createElement('s'); FD.body.appendChild(s); return [s.ownerDocument === FD, s instanceof HTMLElement].join(); });
resolve(out);
