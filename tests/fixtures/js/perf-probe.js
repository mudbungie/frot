// The resource-timing oracle (bl-c378): every answer here was read on Firefox
// 153.0esr (identity.md §3.19 rig, served over http: as /probe.js, inserted by
// the page) and captured in perf-firefox153.json; js::tests::perf serves the
// same files to frot and requires the same answers. Values that are network
// timing (which differ run to run) are compared only as relations.
var out = {};
function E(x) { return x && x.name ? x.name + ': ' + x.message : String(x); }
function desc(o, k) { var d = Object.getOwnPropertyDescriptor(o, k); if (!d) return null; var r = (d.get ? 'g' : '') + (d.set ? 's' : '') + ('value' in d ? 'v' : '') + (d.enumerable ? 'E' : '') + (d.writable ? 'W' : '') + (d.configurable ? 'C' : ''); if (typeof d.value === 'function') r += ':' + d.value.length; return r; }
['PerformanceEntry', 'PerformanceResourceTiming'].forEach(function (n) {
  var C = window[n], P = C.prototype, parent = Object.getPrototypeOf(C);
  out['iface ' + n] = JSON.stringify({ len: C.length, parent: parent && parent.name, proto: Object.getOwnPropertyNames(P).map(function (k) { return k + ' ' + desc(P, k); }), tag: String(P[Symbol.toStringTag]), protoParent: Object.getPrototypeOf(P) === Object.prototype ? 'Object' : Object.getPrototypeOf(P).constructor.name });
  try { new C(); out['new ' + n] = 'ok'; } catch (x) { out['new ' + n] = E(x); }
});
var s = document.createElement('script'); s.src = '/ext.js';
s.onload = function () {
  fetch('/s.css').then(function (r) { return r.text(); }).then(function () {
    var x = new XMLHttpRequest(); x.open('GET', '/x.css'); x.onload = function () { setTimeout(fin, 50); }; x.send();
  });
};
document.head.appendChild(s);
function mine(e) { return !/favicon/.test(e.name); }
function fin() {
  var res = performance.getEntriesByType('resource').filter(mine);
  out.types = res.map(function (e) { return e.entryType + ':' + e.initiatorType + ':' + e.name.replace(location.origin, ''); }).join(' | ');
  var e = res[res.length - 1];
  out.jsonKeys = Object.keys(e.toJSON()).join();
  out.own = Object.getOwnPropertyNames(e).join();
  out.tag = Object.prototype.toString.call(e);
  out.same = (performance.getEntriesByType('resource')[0] === performance.getEntriesByType('resource')[0]) + ',' + (performance.getEntries().filter(function (e) { return e.entryType === 'resource' && mine(e); })[0] === res[0]);
  out.inst = (e instanceof PerformanceResourceTiming) + ',' + (e instanceof PerformanceEntry);
  out.serverTiming = Object.prototype.toString.call(e.serverTiming) + e.serverTiming.length + (e.serverTiming === e.serverTiming) + Object.isFrozen(e.serverTiming);
  out.stringifyStart = JSON.stringify(e).replace(location.origin, '').slice(0, 45);
  out.entryToJSONThis = (function () { try { return PerformanceEntry.prototype.toJSON.call({}); } catch (x) { return E(x); } })();
  out.prtToJSONThis = (function () { try { return PerformanceResourceTiming.prototype.toJSON.call({}); } catch (x) { return E(x); } })();
  out.getName = (function () { try { return Object.getOwnPropertyDescriptor(PerformanceEntry.prototype, 'name').get.call({}); } catch (x) { return E(x); } })();
  out.getInit = (function () { try { return Object.getOwnPropertyDescriptor(PerformanceResourceTiming.prototype, 'initiatorType').get.call({}); } catch (x) { return E(x); } })();
  out.getNameName = Object.getOwnPropertyDescriptor(PerformanceEntry.prototype, 'name').get.name;
  out.byName = performance.getEntriesByName(e.name).length + ',' + performance.getEntriesByName(e.name, 'resource').length + ',' + performance.getEntriesByName(e.name, 'mark').length;
  out.byNameArgs = (function () { try { return performance.getEntriesByName(); } catch (x) { return E(x); } })();
  out.byTypeArgs = (function () { try { return performance.getEntriesByType(); } catch (x) { return E(x); } })();
  out.order = (function () { var a = performance.getEntries(); for (var i = 1; i < a.length; i++) if (a[i].startTime < a[i - 1].startTime) return 'unsorted'; return 'sorted'; })();
  out.values = res.map(function (e) { return [e.initiatorType, e.responseStatus, e.contentType, e.fetchStart === e.startTime, e.responseEnd === e.startTime + e.duration, e.workerStart, e.redirectStart, e.redirectEnd, e.secureConnectionStart, e.firstInterimResponseStart, typeof e.nextHopProtocol, typeof e.transferSize].join('/'); }).join(' | ');
  out.listShape = (function () { var got; var o = new PerformanceObserver(function (l) { got = l.getEntries(); }); o.observe({ type: 'resource', buffered: true }); var t = o.takeRecords(); o.disconnect(); return t.filter(mine).length + ',' + (t.filter(mine)[0] === res[0]); })();
  done(out);
}
