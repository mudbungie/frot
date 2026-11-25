// The HTMLScriptElement oracle (bl-f9f2): read on Firefox 153.0esr (identity.md
// §3.19 rig), served as /probe.js from a page whose two parser scripts are
// #pa (async) and #pd (defer); answers in script-firefox153.json, which
// js::tests::scriptprops requires of frot.
var out = {};
function E(x) { return x && x.name ? x.name + ': ' + x.message : String(x); }
function T(k, f) { try { out[k] = String(f()); } catch (x) { out[k] = 'THROW ' + E(x); } }
var props = ['src', 'type', 'noModule', 'charset', 'async', 'defer', 'crossOrigin', 'referrerPolicy', 'text', 'fetchPriority', 'event', 'htmlFor', 'integrity'];
T('proto', function () { return Object.getOwnPropertyNames(HTMLScriptElement.prototype).join(); });
T('statics', function () { return Object.getOwnPropertyNames(HTMLScriptElement).join() + '|' + typeof HTMLScriptElement.supports + '|' + [HTMLScriptElement.supports('classic'), HTMLScriptElement.supports('module'), HTMLScriptElement.supports('importmap'), HTMLScriptElement.supports('speculationrules'), HTMLScriptElement.supports('x')].join(); });
T('defaults', function () { var s = document.createElement('script'); return props.map(function (p) { return p + '=' + JSON.stringify(s[p]); }).join(' '); });
T('parserDefaults', function () { var s = document.scripts[0]; return props.filter(function (p) { return p !== 'text' && p !== 'src'; }).map(function (p) { return p + '=' + JSON.stringify(s[p]); }).join(' '); });
T('asyncFlag', function () { var s = document.createElement('script'); var a = [s.async]; s.async = false; a.push(s.async, s.hasAttribute('async')); s.async = true; a.push(s.async, s.getAttribute('async')); s.async = false; a.push(s.async, s.hasAttribute('async')); return a.join(); });
T('asyncAttrRemove', function () { var s = document.createElement('script'); s.setAttribute('async', ''); var a = [s.async]; s.removeAttribute('async'); a.push(s.async); return a.join(); });
T('asyncClone', function () { var s = document.createElement('script'); s.async = false; var c = s.cloneNode(); var d = document.createElement('script').cloneNode(); return [c.async, d.async].join(); });
T('innerHTMLAsync', function () { var d = document.createElement('div'); d.innerHTML = '<script></script>'; return d.firstChild.async; });
T('parserAsyncAttr', function () { return document.getElementById('pa').async + ',' + document.getElementById('pd').defer; });
T('crossOrigin', function () { var s = document.createElement('script'); var a = [s.crossOrigin]; ['', 'anonymous', 'use-credentials', 'USE-CREDENTIALS', 'x'].forEach(function (v) { s.setAttribute('crossorigin', v); a.push(s.crossOrigin); }); s.crossOrigin = null; a.push(s.hasAttribute('crossorigin'), s.crossOrigin); s.crossOrigin = 'Foo'; a.push(s.getAttribute('crossorigin'), s.crossOrigin); return a.join('|'); });
T('referrerPolicy', function () { var s = document.createElement('script'); var a = [JSON.stringify(s.referrerPolicy)]; ['no-referrer', 'NO-REFERRER', 'x', 'unsafe-url', 'strict-origin-when-cross-origin', 'origin', 'same-origin', 'origin-when-cross-origin', 'no-referrer-when-downgrade', 'strict-origin'].forEach(function (v) { s.setAttribute('referrerpolicy', v); a.push(s.referrerPolicy); }); s.referrerPolicy = 'Bad'; a.push(s.getAttribute('referrerpolicy')); return a.join('|'); });
T('fetchPriority', function () { var s = document.createElement('script'); var a = [s.fetchPriority]; ['high', 'LOW', 'auto', 'x', ''].forEach(function (v) { s.setAttribute('fetchpriority', v); a.push(s.fetchPriority); }); s.fetchPriority = 'Zz'; a.push(s.getAttribute('fetchpriority')); return a.join('|'); });
T('strings', function () { var s = document.createElement('script'); s.charset = 'utf-8'; s.event = 'onload'; s.htmlFor = 'w'; s.integrity = 'sha256-x'; s.noModule = true; s.defer = 1; return [s.getAttribute('charset'), s.getAttribute('event'), s.getAttribute('for'), s.getAttribute('integrity'), JSON.stringify(s.getAttribute('nomodule')), JSON.stringify(s.getAttribute('defer')), s.noModule, s.defer].join('|'); });
T('typeAttr', function () { var s = document.createElement('script'); s.type = 'module'; var a = [s.getAttribute('type')]; s.removeAttribute('type'); a.push(JSON.stringify(s.type)); return a.join('|'); });
T('srcRel', function () { var s = document.createElement('script'); s.setAttribute('src', 'a.js'); return s.src.replace(location.origin, ''); });
T('descs', function () { return props.map(function (p) { var d = Object.getOwnPropertyDescriptor(HTMLScriptElement.prototype, p); return p + ':' + (d.get ? 'g' : '') + (d.set ? 's' : '') + (d.enumerable ? 'e' : '') + (d.configurable ? 'c' : ''); }).join(' '); });
T('inOp', function () { return ['async' in document.createElement('script'), 'noModule' in HTMLScriptElement.prototype].join(); });
done(out);
