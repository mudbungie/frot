// PerformanceObserver and PerformanceObserverEntryList (bl-b129): Cloudflare's
// managed challenge constructs one for ['resource', 'navigation'] and threw
// `not a function` while the name was absent. Shape and errors are read off
// Firefox 153.0esr (identity.md §3.20).
//
// What is delivered is what frot measures: `resource` entries, the same ones
// `performance.getEntriesByType('resource')` answers, queued as a task once the
// request has completed. `supportedEntryTypes` is the binary's list; the other
// seven types are accepted and never delivered, each for a stated reason
// (identity.md §11) — frot measures no document fetch (navigation), paints
// nothing (paint, largest-contentful-paint), synthesizes no input (event,
// first-input) and has no `performance.mark`/`measure`. Runs after loop.js,
// through its `__frot_perf_hook` seam, which this module removes.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var hook = g.__frot_perf_hook, URL = g.URL;
  delete g.__frot_perf_hook;

  // --- PerformanceEntry and PerformanceResourceTiming (bl-c378) --------------
  // Interfaces, members and order read off Firefox 153.0esr (js.md §5). An
  // entry owns nothing; its values are slots. What frot measures, it says:
  // startTime/duration (and fetchStart/responseEnd, their two ends), who made
  // the request, and — for a resource of the document's own origin — its status
  // and MIME type, which Firefox gives such a resource (a cross-origin one
  // reads 0 and ''). Every other value is the 0, '' or [] Firefox reports when a
  // resource's timing details are withheld from the page: frot measures no DNS,
  // connection, TLS or transfer phase and no sizes, and invents none.
  var TIMING = ['initiatorType', 'nextHopProtocol', 'workerStart', 'redirectStart',
    'redirectEnd', 'fetchStart', 'domainLookupStart', 'domainLookupEnd', 'connectStart',
    'connectEnd', 'secureConnectionStart', 'requestStart', 'finalResponseHeadersStart',
    'firstInterimResponseStart', 'responseStart', 'responseEnd', 'transferSize',
    'encodedBodySize', 'decodedBodySize', 'responseStatus', 'contentType', 'serverTiming'];
  function own(entry, member, name) {
    var st = slots(entry);
    if (st.entryType !== 'resource') {
      throw new TypeError("'" + member + "' called on an object that does not implement "
        + 'interface ' + name + '.');
    }
    return st;
  }
  function getters(names, name) {
    var out = {};
    names.forEach(function (k) {
      out[k] = function () {
        return own(this, 'get ' + k, name)[k];
      };
    });
    return out;
  }
  var Entry = g.__frot_iface('PerformanceEntry');
  g.__frot_ifaceops(Entry.prototype, { toJSON: function toJSON() {
    var st = own(this, 'toJSON', 'PerformanceEntry');
    return { name: st.name, entryType: st.entryType, startTime: st.startTime,
      duration: st.duration };
  } });
  g.__frot_ifaceattrs(Entry.prototype,
    getters(['name', 'entryType', 'startTime', 'duration'], 'PerformanceEntry'));
  var Resource = g.__frot_extends(g.__frot_iface('PerformanceResourceTiming'), Entry);
  g.__frot_ifaceops(Resource.prototype, { toJSON: function toJSON() {
    var st = own(this, 'toJSON', 'PerformanceResourceTiming');
    var o = Entry.prototype.toJSON.call(this);
    TIMING.forEach(function (k) {
      o[k] = k === 'serverTiming' ? [] : st[k];
    });
    return o;
  } });
  g.__frot_ifaceattrs(Resource.prototype, getters(TIMING, 'PerformanceResourceTiming'));
  // net.js names an XMLHttpRequest's entries (the host knows only "a page
  // fetch"), by index, through `__frot_initiator`, which it takes off the global.
  var labels = {};
  Object.defineProperty(g, '__frot_initiator', { configurable: true, value: function (i, type) {
    labels[i] = type;
  } });
  // `raw` is the host's [name, start, duration, initiator, status, contentType].
  hook.entry = function (raw, i, floor) {
    var e = Object.create(Resource.prototype), s = floor(raw[1]), d = floor(raw[2]);
    var mine = new URL(raw[0]).origin === g.__frot_location().origin;
    var st = Object.assign(slots(e), { name: raw[0], entryType: 'resource', startTime: s,
      duration: d, serverTiming: Object.freeze([]) });
    TIMING.forEach(function (k) {
      if (k !== 'serverTiming') st[k] = 0;
    });
    Object.assign(st, { initiatorType: labels[i] || raw[3], nextHopProtocol: '', fetchStart: s,
      responseEnd: s + d, responseStatus: mine ? raw[4] : 0,
      contentType: mine ? raw[5].split(';')[0].trim().toLowerCase() : '' });
    return e;
  };

  var SUPPORTED = ['event', 'first-input', 'largest-contentful-paint', 'mark', 'measure',
    'navigation', 'paint', 'resource'];
  var active = []; // observers that asked for `resource`
  var seen = 0; // resource timings already handed to them

  var List = g.__frot_iface('PerformanceObserverEntryList');
  g.__frot_ifaceops(List.prototype, {
    getEntries: function getEntries() {
      return slots(this).entries.slice();
    },
    getEntriesByType: function getEntriesByType(type) {
      return slots(this).entries.filter(function (e) {
        return e.entryType === type;
      });
    },
    getEntriesByName: function getEntriesByName(name) {
      return slots(this).entries.filter(function (e) {
        return e.name === name;
      });
    },
  });

  // One task per observer with records waiting: callback(list, observer), with
  // the observer as `this` (measured). A throw is an uncaught error (§10).
  function deliver(o) {
    var st = slots(o);
    st.scheduled = false;
    var list = Object.create(List.prototype);
    slots(list).entries = st.records.splice(0);
    if (!slots(list).entries.length) return;
    try {
      st.cb.call(o, list, o);
    } catch (e) {
      g.reportError(e);
    }
  }
  function queue(o, entries) {
    var st = slots(o);
    st.records.push.apply(st.records, entries);
    if (st.scheduled || !entries.length) return;
    st.scheduled = true;
    hook.schedule(deliver, 0, [o], 0);
  }
  // Asked by the loop before it picks each task, while anyone is observing.
  function pump() {
    var all = hook.entries();
    var fresh = all.slice(seen);
    seen = all.length;
    active.forEach(function (o) {
      queue(o, fresh);
    });
  }
  function stop(o) {
    active = active.filter(function (a) {
      return a !== o;
    });
    if (!active.length) hook.before = null;
  }

  var at = 'PerformanceObserver.observe: ';
  var Observer = g.__frot_iface('PerformanceObserver', null, function (inst, args) {
    var ctor = 'PerformanceObserver constructor: ';
    if (!args.length) throw new TypeError(ctor + 'At least 1 argument required, but only 0 passed');
    if (typeof args[0] !== 'function') {
      var what = args[0] !== null && typeof args[0] === 'object' ? 'callable.' : 'an object.';
      throw new TypeError(ctor + 'Argument 1 is not ' + what);
    }
    var st = slots(inst);
    st.cb = args[0];
    st.records = [];
  });
  Object.defineProperty(Observer, 'length', { value: 1, configurable: true });
  g.__frot_ifaceops(Observer.prototype, {
    observe: function observe(options) {
      var o = options || {};
      var st = slots(this);
      var many = o.entryTypes !== undefined;
      if (!many && o.type === undefined) {
        throw new TypeError(at + "Can't call observe without `type` or `entryTypes`");
      }
      if (many && o.type !== undefined) {
        throw new TypeError(at + "Can't call observe with both `type` and `entryTypes`");
      }
      if (st.many !== undefined && st.many !== many) {
        // The binary's own message, cut short there too.
        throw g.__frot_domerror('InvalidModificationError',
          'An attempt was made to modify the type of the underlying objec');
      }
      st.many = many;
      var types = many ? Array.prototype.map.call(o.entryTypes, String) : [String(o.type)];
      if (types.indexOf('resource') < 0 || active.indexOf(this) >= 0) return;
      // Entries from before this moment belong to nobody unless one observer
      // asks for them: `{type, buffered: true}` (measured).
      if (!active.length) seen = hook.entries().length;
      active.push(this);
      hook.before = pump;
      if (!many && o.buffered) queue(this, hook.entries().slice(0, seen));
    },
    disconnect: function disconnect() {
      slots(this).records = [];
      stop(this);
    },
    takeRecords: function takeRecords() {
      return slots(this).records.splice(0);
    },
  });
  // A fresh array at every read (measured: not frozen, not the same object).
  Object.defineProperty(Observer, 'supportedEntryTypes', {
    get: g.__frot_brand(function () {
      return SUPPORTED.slice();
    }, 'get supportedEntryTypes'),
    enumerable: true,
    configurable: true,
  });
})(globalThis);
