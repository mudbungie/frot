// srcdoc probe (bl-7b86): run on Firefox 153.0esr (identity.md §3.19 rig) over
// srcdoc/page.html; the log it resolves is kept in firefox153.json.
var L = window.L = [];
var f = document.createElement('iframe');
f.srcdoc = '<!DOCTYPE html><title>T</title><p id=x>hi</p><script>parent.L.push("script:" + location.href + ":" + document.readyState + ":" + document.title + ":" + (typeof parent.f) + ":" + document.getElementById("x").textContent + ":" + document.compatMode); window.addEventListener("DOMContentLoaded", function () { parent.L.push("f-dcl:" + document.readyState); }); window.addEventListener("load", function () { parent.L.push("f-load:" + document.readyState); });<\/script><script>parent.L.push("script2:" + (document.scripts.length));<\/script>';
window.f = f;
f.addEventListener('load', function () { L.push('iframe-load:' + f.contentWindow.location.href + ':' + (f.contentWindow === W0) + ':' + f.contentDocument.body.innerHTML.length + ':' + (f.contentDocument === D0)); });
document.body.appendChild(f);
var W0 = f.contentWindow, D0 = f.contentDocument;
L.push('sync:' + W0.location.href + ':' + D0.readyState + ':' + D0.body.innerHTML.length);
W0.marker = 1;
setTimeout(function () { L.push('t0:' + f.contentWindow.location.href); }, 0);
Promise.resolve().then(function () { L.push('micro'); });
setTimeout(function () {
  L.push('after:' + (f.contentWindow === W0) + ':' + typeof f.contentWindow.marker + ':' + (f.contentDocument === D0) + ':' + f.contentDocument.documentElement.outerHTML.length + ':' + f.contentDocument.title);
  var g = document.createElement('iframe'); g.setAttribute('srcdoc', '<script>parent.L.push("parsed-attr-run")<\/script>'); document.body.appendChild(g);
  setTimeout(function () { g.srcdoc = '<script>parent.L.push("changed-run")<\/script>'; setTimeout(function () { g.removeAttribute('srcdoc'); setTimeout(function () { L.push('removed:' + g.contentWindow.location.href); var h = document.createElement('iframe'); h.srcdoc = '<script>parent.L.push("err-next");throw new Error("sd-boom")<\/script>'; document.body.appendChild(h); setTimeout(function () { resolve({ log: L.join('\n') }); }, 200); }, 200); }, 200); }, 200);
}, 300);
