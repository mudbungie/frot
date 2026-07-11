// Network (js.md §6) — fetch + XMLHttpRequest over the once-then-frozen
// __frot_subfetch syscall. GET only: any other method rejects (fetch) / throws
// (XHR), counted through the §10 unified error channel. A refused or failed
// request rejects/errors the same way. No live network after a URL first
// resolves; nothing persists past the call. Loads after loop.js (uses g.Event).
(function (g) {
  'use strict';

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
  function Headers(pairs) {
    this._m = Object.create(null);
    for (var i = 0; i < (pairs || []).length; i++)
      this._m[String(pairs[i][0]).toLowerCase()] = pairs[i][1];
  }
  Headers.prototype.get = function (n) {
    n = String(n).toLowerCase();
    return n in this._m ? this._m[n] : null;
  };
  Headers.prototype.has = function (n) {
    return String(n).toLowerCase() in this._m;
  };

  // --- Response: the read subset over a frozen body ---------------------------
  function Response(r) {
    this.ok = !!r.ok;
    this.status = r.status;
    this.statusText = '';
    this.url = r.url;
    this.headers = new Headers(r.headers);
    this.bodyUsed = false;
    this._body = r.body;
  }
  Response.prototype.text = function () {
    return Promise.resolve(this._body);
  };
  Response.prototype.json = function () {
    var body = this._body;
    // A parse error rejects the promise (executor throw), as the spec requires.
    return new Promise(function (resolve) {
      resolve(JSON.parse(body));
    });
  };
  Response.prototype.clone = function () {
    return new Response({ ok: this.ok, status: this.status, url: this.url, body: this._body });
  };

  g.Headers = Headers;
  g.Response = Response;
  g.Request = function (input, init) {
    this.url = absolute(input);
    this.method = method(init);
  };

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
  function XHR() {
    this.readyState = 0;
    this.status = 0;
    this.statusText = '';
    this.responseText = '';
    this.response = '';
    this.onreadystatechange = null;
    this.onload = null;
    this.onerror = null;
    this._method = 'GET';
    this._url = '';
    this._headers = [];
  }
  XHR.prototype.open = function (m, url) {
    this._method = String(m).toUpperCase();
    this._url = String(url);
    this.readyState = 1;
  };
  XHR.prototype.setRequestHeader = function () {};
  XHR.prototype.abort = function () {};
  XHR.prototype.getAllResponseHeaders = function () {
    return this._headers
      .map(function (h) {
        return h[0].toLowerCase() + ': ' + h[1];
      })
      .join('\r\n');
  };
  XHR.prototype.getResponseHeader = function (n) {
    n = String(n).toLowerCase();
    for (var i = 0; i < this._headers.length; i++)
      if (String(this._headers[i][0]).toLowerCase() === n) return this._headers[i][1];
    return null;
  };
  XHR.prototype.addEventListener = function (type, fn) {
    if (type === 'load') this.onload = fn;
    else if (type === 'error') this.onerror = fn;
  };
  XHR.prototype.removeEventListener = function () {};
  XHR.prototype._done = function () {
    this.readyState = 4;
    if (typeof this.onreadystatechange === 'function') this.onreadystatechange();
  };
  XHR.prototype.send = function () {
    // GET only: a non-GET send throws (counted), never submits (§6).
    if (this._method !== 'GET') throw new Error('frot: only GET requests are supported');
    var r = g.__frot_subfetch(this._url);
    if (r.error) {
      this.status = 0;
      this._done();
      if (typeof this.onerror === 'function') this.onerror(new g.Event('error'));
      return;
    }
    this.status = r.status;
    this.responseText = r.body;
    this.response = r.body;
    this._headers = r.headers || [];
    this._done();
    if (typeof this.onload === 'function') this.onload(new g.Event('load'));
  };
  g.XMLHttpRequest = XHR;
})(globalThis);
