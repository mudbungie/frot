// Network (js.md §6) — fetch + XMLHttpRequest over the once-then-frozen
// __frot_subfetch syscall (GET) and __frot_subpost (the page's own POST,
// challenge.md §2/§3 — never frozen). This file is the ONLY place frot names an
// HTTP verb: GET and POST go to the wire, any other method rejects (fetch) /
// throws (XHR), counted through the §10 unified error channel. A POST carries
// the text body the script gave (a string, a URLSearchParams, or a FormData
// as multipart text — any other body type fails the request) and its Content-Type; nothing else the script set
// rides along. A refused or failed request rejects/errors the same way. No live network after a URL first
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
  // formdata.js's serializer, taken off the global: a page never sees it.
  var multipart = g.__frot_multipart;
  delete g.__frot_multipart;
  // response.js's maker for a fetched Response, taken off the global here.
  var make = g.__frot_respond;
  delete g.__frot_respond;
  // perfobserver.js's namer for an XMLHttpRequest's resource entry (bl-c378).
  var initiator = g.__frot_initiator;
  delete g.__frot_initiator;

  // A Request-like input exposes `.url`; anything else is stringified.
  function absolute(input) {
    return input && typeof input === 'object' && 'url' in input
      ? String(input.url)
      : String(input);
  }
  function method(init) {
    return init && init.method ? String(init.method).toUpperCase() : 'GET';
  }

  // The one dispatch: a GET through the frozen cache, or the script's POST
  // with its text body. `ctype` is the script's own Content-Type, else what a
  // browser derives from the body's type.
  function dispatch(m, url, body, ctype) {
    if (m === 'GET') return g.__frot_subfetch(url);
    var form = body instanceof g.URLSearchParams;
    var none = body === undefined || body === null;
    // A FormData goes out as the multipart text Firefox serializes (bl-6eb9).
    var parts = none ? null : multipart(body);
    if (parts && parts.error) return parts;
    // A text body only: bytes frot cannot carry faithfully (an ArrayBuffer, a
    // Blob) fail the request rather than go out as something else.
    if (!none && !form && !parts && typeof body !== 'string')
      return { error: 'frot: only text request bodies are supported' };
    var kind = form ? 'application/x-www-form-urlencoded;charset=UTF-8' : 'text/plain;charset=UTF-8';
    if (parts) kind = parts.type;
    var text = none ? '' : parts ? parts.body : String(body);
    return g.__frot_subpost(url, text, ctype ? String(ctype) : kind);
  }
  function supported(m) {
    return m === 'GET' || m === 'POST';
  }
  // The Content-Type among a fetch `init.headers` in any of its three shapes:
  // a Headers, a list of pairs, a plain record.
  function contentType(h) {
    if (!h) return null;
    if (typeof h.get === 'function') return h.get('content-type');
    var pairs = Array.isArray(h) ? h : Object.keys(h).map(function (k) { return [k, h[k]]; });
    for (var i = 0; i < pairs.length; i++)
      if (String(pairs[i][0]).toLowerCase() === 'content-type') return pairs[i][1];
    return null;
  }

  // A Request owns nothing either; measured prototype puts `method` and `url`
  // among the attributes, method first.
  var Request = iface('Request', null, function (inst, args) {
    var st = slots(inst);
    st.method = method(args[1]);
    st.url = absolute(args[0]);
  });
  attrs(Request.prototype, ['method', 'url']);

  g.fetch = function (input, init) {
    var m = method(init);
    if (!supported(m))
      return Promise.reject(new TypeError('frot: only GET and POST requests are supported'));
    var r = dispatch(m, absolute(input), init && init.body, contentType(init && init.headers));
    if (r.error) return Promise.reject(new TypeError(r.error));
    return Promise.resolve(make(r));
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
  ['onloadstart', 'onprogress', 'onabort', 'onerror', 'onload', 'ontimeout', 'onloadend'].forEach(
    function (k) {
      g.__frot_onevent(XHRTarget.prototype, k);
    }
  );
  // An event the browser raises on an XHR: dispatched, so `onload` (a
  // listener in its registration place) and `addEventListener` both hear it.
  function emit(xhr, Ctor, type) {
    var ev = new Ctor(type);
    slots(ev).trusted = true;
    xhr.dispatchEvent(ev);
  }

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
    st.ctype = null;
  });
  Object.setPrototypeOf(XHR.prototype, XHRTarget.prototype);
  ops(XHR.prototype, {
    open: function open(m, url) {
      var st = slots(this);
      st.method = String(m).toUpperCase();
      st.url = String(url);
      st.ctype = null;
      st.readyState = 1;
    },
    // Only the Content-Type of the script's own POST body is carried (§6).
    setRequestHeader: function setRequestHeader(n, v) {
      if (String(n).toLowerCase() === 'content-type') slots(this).ctype = String(v);
    },
    send: function send() {
      // Any method but GET/POST throws (counted), never reaches the wire (§6).
      var st = slots(this);
      if (!supported(st.method))
        throw new Error('frot: only GET and POST requests are supported');
      var n = __frot_resource_timings().length;
      var r = dispatch(st.method, st.url, arguments[0], st.ctype);
      // A request that reached the network is this XHR's entry (Firefox:
      // initiatorType 'xmlhttprequest'); a cache hit made none.
      if (__frot_resource_timings().length > n) initiator(n, 'xmlhttprequest');
      st.status = r.error ? 0 : r.status;
      if (!r.error) {
        st.responseText = r.body;
        st.response = r.body;
        st.headers = r.headers || [];
      }
      st.readyState = 4;
      emit(this, g.Event, 'readystatechange');
      emit(this, g.ProgressEvent, r.error ? 'error' : 'load');
      emit(this, g.ProgressEvent, 'loadend');
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
