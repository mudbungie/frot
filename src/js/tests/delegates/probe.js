// delegatesFocus and focus in shadow trees (js.md §15.11): run on Firefox
// 153.0esr (identity.md §3.19 rig) over delegates/page.html, its answers kept
// in firefox153.json.
var out = {};
var $ = function (id) { return document.getElementById(id); };
var nm = function (n) { return n === null ? 'null' : n === undefined ? 'undef' : n === document.body ? 'body' : n.id || n.localName; };
var t = function (k, f) { try { var v = f(); out[k] = v === undefined ? 'undefined' : v; } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var root = function (h, init, html) { var r = h.attachShadow(init); r.innerHTML = html; return r; };
var rd = root($('d'), { mode: 'open', delegatesFocus: true }, '<span>text</span><input id=di1><input id=di2>');
var rn = root($('n'), { mode: 'open' }, '<input id=ni>');
var rp = root($('p'), { mode: 'closed', delegatesFocus: true }, '<p>none focusable</p>');
var ra = root($('a'), { mode: 'open', delegatesFocus: true }, '<input id=ai1><input id=ai2 autofocus>');
var state = function () { return [nm(document.activeElement), nm(rd.activeElement), nm(rn.activeElement), $('d').matches(':focus'), $('d').matches(':focus-within'), $('out').matches(':focus-within')].join(' '); };
t('initial', function () { return state() + ' ' + rd.delegatesFocus; });
t('hostFocus', function () { $('d').focus(); return state(); });
t('inner', function () { return nm(rd.activeElement) + ' ' + rd.getElementById('di1').matches(':focus') + ' ' + document.querySelector(':focus') + ' ' + nm(rd.querySelector(':focus')); });
t('again', function () { rd.getElementById('di2').focus(); $('d').focus(); return state(); });
t('noDelegate', function () { $('n').focus(); return state() + ' ' + $('n').matches(':focus'); });
t('innerOfPlain', function () { rn.getElementById('ni').focus(); return state() + ' ' + $('n').matches(':focus') + ' ' + nm(rn.activeElement); });
t('nothing', function () { $('plain').focus(); $('p').focus(); return state() + ' ' + nm(document.activeElement); });
t('autofocus', function () { $('a').focus(); return nm(document.activeElement) + ' ' + nm(ra.activeElement); });
t('blurHost', function () { $('a').blur(); return nm(document.activeElement) + ' ' + nm(ra.activeElement); });
t('innerBlur', function () { $('d').focus(); rd.getElementById('di1').blur(); return nm(document.activeElement); });
t('blurPlainHost', function () { rn.getElementById('ni').focus(); $('n').blur(); return nm(document.activeElement) + ' ' + nm(rn.activeElement); });
t('focusPlainHostInside', function () { rn.getElementById('ni').focus(); $('n').focus(); return nm(document.activeElement) + ' ' + nm(rn.activeElement); });
resolve(out);
