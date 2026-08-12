// Network (js.md §6) — fetch + XMLHttpRequest over the once-then-frozen
// __frot_subfetch syscall. GET only: any other method rejects (fetch) / throws
// (XHR), counted through the §10 unified error channel. A refused or failed
// request rejects/errors the same way. No live network after a URL first
// resolves; nothing persists past the call. Loads after events.js (uses g.Event
// and its EventTarget).
//
// The four objects here are real interfaces (bl-643d). They were hand-rolled
// constructors that stamped every value onto the instance, so a page walking a
// Response saw six own data properties and an XHR saw eight — where a real one
// owns NOTHING and answers off its prototype. Member lists and Gecko's member
// order read off Firefox 153.0esr (identity.md §3.17).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var iface = g.__frot_iface;
  var attrs = g.__frot_ifaceattrs;
  var ops = g.__frot_ifaceops;

  // A Request-like input exposes `.url`; anything else is stringified.
  function absolute(input) {
    return input && typeof input === 'object' && 'url' in input
      ? String(input.url)
      : String(input);
  }
  function method(init) {
    return init && init.method ? String(init.method).toUpperCase() : 'GET';
  }

  // --- Headers: the case-insensitive read subset the shim answers -------------
  // Measured order: append, delete, get, getSetCookie, has, set, entries, keys,
  // values, forEach. frot answers `get` and `has`; the rest are absent rather
  // than faked, so feature detection sees what frot can actually do.
  var Headers = iface('Headers', null, function (inst, args) {
    var m = Object.create(null);
    var pairs = args[0] || [];
    for (var i = 0; i < pairs.length; i++)
      m[String(pairs[i][0]).toLowerCase()] = pairs[i][1];
    slots(inst).m = m;
  });
  ops(Headers.prototype, {
    get: function get(n) {
      var m = slots(this).m;
      n = String(n).toLowerCase();
      return n in m ? m[n] : null;
    },
    has: function has(n) {
      return String(n).toLowerCase() in slots(this).m;
    },
  });

  // --- Response: the read subset over a frozen body ---------------------------
  // Measured order: clone, arrayBuffer, blob, bytes, formData, json, text, then
  // type, url, redirected, status, ok, statusText, headers, body, bodyUsed.
  // `new Response(body)` is 200 / ok / statusText '' — measured on the binary,
  // where a fresh Response reports status 200 and type 'default'.
  var Response = iface('Response', null, function (inst, args) {
    var r = args[0];
    var st = slots(inst);
    var given = r !== null && typeof r === 'object' && 'status' in r;
    st.ok = given ? !!r.ok : true;
    st.status = given ? r.status : 200;
    st.statusText = '';
    st.url = given ? r.url : '';
    st.headers = new Headers(given ? r.headers : []);
    st.bodyUsed = false;
    st.body = given ? r.body : (r === undefined ? null : String(r));
  });
  ops(Response.prototype, {
    clone: function clone() {
      return new Response({
        ok: this.ok, status: this.status, url: this.url, body: slots(this).body,
        headers: [],
      });
    },
    json: function json() {
      var body = slots(this).body;
      // A parse error rejects the promise (executor throw), as the spec requires.
      return new Promise(function (resolve) {
        resolve(JSON.parse(body));
      });
    },
    text: function text() {
      return Promise.resolve(slots(this).body);
    },
  });
  attrs(Response.prototype, ['url', 'status', 'ok', 'statusText', 'headers', 'bodyUsed']);

  // A Request owns nothing either; measured prototype puts `method` and `url`
  // among the attributes, method first.
  var Request = iface('Request', null, function (inst, args) {
    var st = slots(inst);
    st.method = method(args[1]);
    st.url = absolute(args[0]);
  });
  attrs(Request.prototype, ['method', 'url']);

  g.fetch = function (input, init) {
    if (method(init) !== 'GET')
      return Promise.reject(new TypeError('frot: only GET requests are supported'));
    var r = g.__frot_subfetch(absolute(input));
    if (r.error) return Promise.reject(new TypeError(r.error));
    return Promise.resolve(new Response(r));
  };

  // --- XMLHttpRequest: a synchronous GET subset (§6: the loop is single-
  // threaded, so a blocking GET is trivially "sync"). open/send/status/
  // responseText/onload/onerror/onreadystatechange, plus response-header reads.
  //
  // Two interfaces on the binary, not one: the `on*` handlers live on
  // `XMLHttpRequestEventTarget.prototype`, which XMLHttpRequest inherits and
  // which itself inherits EventTarget — so `addEventListener` is the one
  // registry's, not a third private copy. The four ready-state constants sit on
  // both the prototype and the constructor, non-writable and non-configurable.
  var XHRTarget = iface('XMLHttpRequestEventTarget');
  Object.setPrototypeOf(XHRTarget.prototype, g.EventTarget.prototype);
  ['onabort', 'onerror', 'onload'].forEach(function (k) {
    g.__frot_onevent(XHRTarget.prototype, k);
  });

  var XHR = iface('XMLHttpRequest', null, function (inst) {
    var st = slots(inst);
    st.readyState = 0;
    st.status = 0;
    st.statusText = '';
    st.responseText = '';
    st.response = '';
    st.method = 'GET';
    st.url = '';
    st.headers = [];
  });
  Object.setPrototypeOf(XHR.prototype, XHRTarget.prototype);
  ops(XHR.prototype, {
    open: function open(m, url) {
      var st = slots(this);
      st.method = String(m).toUpperCase();
      st.url = String(url);
      st.readyState = 1;
    },
    setRequestHeader: function setRequestHeader() {},
    send: function send() {
      // GET only: a non-GET send throws (counted), never submits (§6).
      var st = slots(this);
      if (st.method !== 'GET') throw new Error('frot: only GET requests are supported');
      var r = g.__frot_subfetch(st.url);
      st.status = r.error ? 0 : r.status;
      if (!r.error) {
        st.responseText = r.body;
        st.response = r.body;
        st.headers = r.headers || [];
      }
      st.readyState = 4;
      if (typeof st.onreadystatechange === 'function') st.onreadystatechange.call(this);
      var handler = r.error ? this.onerror : this.onload;
      if (typeof handler === 'function') handler.call(this, new g.Event(r.error ? 'error' : 'load'));
    },
    abort: function abort() {},
    getResponseHeader: function getResponseHeader(n) {
      n = String(n).toLowerCase();
      var hs = slots(this).headers;
      for (var i = 0; i < hs.length; i++)
        if (String(hs[i][0]).toLowerCase() === n) return hs[i][1];
      return null;
    },
    getAllResponseHeaders: function getAllResponseHeaders() {
      return slots(this).headers
        .map(function (h) {
          return h[0].toLowerCase() + ': ' + h[1];
        })
        .join('\r\n');
    },
  });
  g.__frot_onevent(XHR.prototype, 'onreadystatechange');
  attrs(XHR.prototype, ['readyState', 'status', 'statusText', 'response', 'responseText']);
  [['UNSENT', 0], ['OPENED', 1], ['HEADERS_RECEIVED', 2], ['LOADING', 3], ['DONE', 4]].forEach(
    function (pair) {
      [XHR.prototype, XHR].forEach(function (o) {
        Object.defineProperty(o, pair[0], {
          value: pair[1], writable: false, enumerable: true, configurable: false,
        });
      });
    }
  );
})(globalThis);
