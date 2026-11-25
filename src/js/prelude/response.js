// Headers and Response (bl-643d; the body, bl-96f7): split out of net.js,
// which runs next and takes `__frot_respond` (a fetched Response's maker) off
// the global. Member lists and Gecko's member order read off Firefox 153.0esr
// (identity.md §3.17).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var iface = g.__frot_iface;
  var attrs = g.__frot_ifaceattrs;
  var ops = g.__frot_ifaceops;
  var multipart = g.__frot_multipart;
  // The stream internals (streams.js and its parts), taken off the global here,
  // their last user.
  var S = g.__frot_streams;
  delete g.__frot_streams;

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

  // --- Response: a frozen body, read through its stream (bl-96f7) ------------
  // Measured order: clone, arrayBuffer, blob, bytes, formData, json, text, then
  // type, url, redirected, status, ok, statusText, headers, body, bodyUsed.
  // `new Response(body)` is 200 / ok / statusText '' (measured). The body is
  // bytes until a page reads `body`, then a byte stream (`Response.body`, a
  // ReadableStream in Firefox); every reader method consumes that stream, so
  // `bodyUsed` is the stream's own "disturbed". `formData` is absent.
  var encoder = new g.TextEncoder(), Blob = g.Blob;
  function utf8(s) {
    return encoder.encode(String(s));
  }
  // BodyInit → bytes (or the page's own stream), and the Content-Type it implies.
  function extract(st, b) {
    var form = multipart(b), type = null;
    if (b === undefined || b === null) st.bytes = null;
    else if (S.isRS(b)) {
      if (S.locked(b) || slots(b).disturbed) {
        throw new TypeError('Response constructor: Body has already been consumed.');
      }
      st.stream = b;
    } else if (slots(b).bytes) {
      st.bytes = slots(b).bytes.slice();
      type = slots(b).type || null;
    }
    else if (b instanceof ArrayBuffer) st.bytes = new Uint8Array(b.slice(0));
    else if (ArrayBuffer.isView(b)) st.bytes = new Uint8Array(b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength));
    else {
      type = form ? form.type : b instanceof g.URLSearchParams
        ? 'application/x-www-form-urlencoded;charset=UTF-8' : 'text/plain;charset=UTF-8';
      st.bytes = utf8(form ? form.body : b);
    }
    st.headers = new Headers(type ? [['content-type', type]] : []);
  }
  function make(r) {
    var res = Object.create(Response.prototype);
    Object.assign(slots(res), { ok: r.ok, status: r.status, statusText: '', url: r.url,
      headers: new Headers(r.headers), bytes: utf8(r.body) });
    return res;
  }
  function bodyOf(res) {
    var st = slots(res);
    if (!st.stream && st.bytes) st.stream = S.bytesStream(st.bytes);
    return st.stream || null;
  }
  function used(res) {
    var b = slots(res).stream;
    return !!b && slots(b).disturbed;
  }
  // Read the whole body: its bytes, or the rejection Firefox gives.
  function consume(res, what) {
    var at = 'Response.' + what + ': ', body = bodyOf(res), chunks = [];
    if (!body) return Promise.resolve(new Uint8Array(0));
    if (used(res)) return Promise.reject(new TypeError(at + 'Body has already been consumed.'));
    if (S.locked(body)) {
      return Promise.reject(new TypeError(at + 'Cannot get a new reader for a readable stream '
        + 'already locked by another reader.'));
    }
    var reader = S.reader(body);
    function pump() {
      var d = S.defer();
      S.read(reader, S.request(d));
      return d.promise.then(function (x) {
        if (x.done) return new Blob(chunks).bytes();
        if (!(x.value instanceof Uint8Array)) {
          throw new TypeError("Can't convert value to Uint8Array while consuming Body");
        }
        chunks.push(x.value);
        return pump();
      });
    }
    return pump();
  }
  function reading(what, shape) {
    return function () {
      return consume(this, what).then(shape);
    };
  }
  var text = reading('text', function (b) {
    return new g.TextDecoder().decode(b);
  });
  var Response = iface('Response', null, function (inst, args) {
    extract(Object.assign(slots(inst), { ok: true, status: 200, statusText: '', url: '' }), args[0]);
  });
  ops(Response.prototype, {
    clone: function clone() {
      if (used(this) || (slots(this).stream && S.locked(slots(this).stream))) {
        throw new TypeError('Response.clone: Body has already been consumed.');
      }
      var st = slots(this), c = Object.create(Response.prototype);
      var cs = Object.assign(slots(c), { ok: st.ok, status: st.status, statusText: st.statusText,
        url: st.url, headers: st.headers, bytes: st.bytes });
      if (st.stream) {
        var two = S.tee(st.stream);
        st.stream = two[0];
        cs.stream = two[1];
      }
      return c;
    },
    arrayBuffer: reading('arrayBuffer', function (b) {
      return b.buffer;
    }),
    blob: function blob() {
      var type = slots(this).headers.get('content-type') || '';
      return consume(this, 'blob').then(function (b) {
        return new Blob([b], { type: type });
      });
    },
    bytes: reading('bytes', function (b) {
      return b;
    }),
    json: function json() {
      return text.call(this).then(JSON.parse);
    },
    text: function text_() {
      return text.call(this);
    },
  });
  attrs(Response.prototype, ['url', 'status', 'ok', 'statusText', 'headers']);
  attrs(Response.prototype, {
    body: function () {
      return bodyOf(this);
    },
    bodyUsed: function () {
      return used(this);
    },
  });

  Object.defineProperty(g, '__frot_respond', { value: make, configurable: true });
})(globalThis);
