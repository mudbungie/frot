// document.title / document.referrer probe (bl-dc63): run on Firefox 153.0esr
// (identity.md §3.19 rig) over title/page.html; answers in firefox153.json.
var out = {};
var t = function (k, f) { try { out[k] = f(); } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var T = function () { return [].map.call(document.querySelectorAll('title'), function (x) { return (x.parentNode ? x.parentNode.nodeName : '-') + ':' + JSON.stringify(x.textContent); }).join('|'); };
t('initial', function () { return JSON.stringify(document.title); });
t('referrer', function () { return JSON.stringify(document.referrer) + typeof document.referrer; });
t('set', function () { document.title = '  x  y '; return JSON.stringify(document.title) + T(); });
t('setNum', function () { document.title = 5; return JSON.stringify(document.title) + T(); });
t('child', function () { var tt = document.querySelector('title'); tt.innerHTML = 'p<b>q</b>r<!--c-->s'; return JSON.stringify(document.title); });
t('removeFirst', function () { document.querySelector('title').remove(); return JSON.stringify(document.title) + T(); });
t('none', function () { [].forEach.call(document.querySelectorAll('title'), function (x) { x.remove(); }); return JSON.stringify(document.title); });
t('setNone', function () { document.title = 'made'; return JSON.stringify(document.title) + T(); });
t('inBody', function () { [].forEach.call(document.querySelectorAll('title'), function (x) { x.remove(); }); var b = document.createElement('title'); b.textContent = ' body t '; document.body.appendChild(b); var r = JSON.stringify(document.title); document.title = 'z'; return r + T(); });
t('noHead', function () { [].forEach.call(document.querySelectorAll('title'), function (x) { x.remove(); }); var h = document.head; h.remove(); document.title = 'nohead'; var r = JSON.stringify(document.title) + T(); document.documentElement.insertBefore(h, document.body); return r; });
t('svgTitle', function () { [].forEach.call(document.querySelectorAll('title'), function (x) { x.remove(); }); document.getElementById('host').innerHTML = '<svg><title>svgt</title></svg>'; var r = JSON.stringify(document.title); document.getElementById('host').innerHTML = ''; return r; });
t('desc', function () { var d = Object.getOwnPropertyDescriptor(Document.prototype, 'title'); var r = Object.getOwnPropertyDescriptor(Document.prototype, 'referrer'); return [!!d.get, !!d.set, d.enumerable, d.get.name, d.set.name, !!r.get, !!r.set, r.get.name].join(); });
t('frame', function () { var f = document.createElement('iframe'); document.body.appendChild(f); var d = f.contentDocument; var r = [JSON.stringify(d.title), JSON.stringify(d.referrer)]; d.title = 'ft'; r.push(JSON.stringify(d.title), d.head.innerHTML); f.remove(); return r.join(); });
t('wsOnly', function () { [].forEach.call(document.querySelectorAll('title'), function (x) { x.remove(); }); document.title = ' \t\n\f\r \u00a0 x\u00a0'; return escape(document.title); });
resolve(out);
