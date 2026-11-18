// Blob, URL.createObjectURL, URL.revokeObjectURL (bl-78c5) — read by AWS WAF's
// challenge (challenge.md §11.4). A Blob is immutable bytes plus a type: a real
// data container, computed in pure JS. Member order, `.length`s, the part and
// type conversions, `slice`'s index arithmetic, the blob: URL format and every
// error message were READ off Firefox 153.0esr (identity.md §3.20).
//
// Declared short of Firefox (identity.md §11):
// - no `stream()` — frot has no ReadableStream;
// - a blob: URL is minted but nothing can LOAD it: `fetch`/XHR of one fails as
//   a network error, and `new Worker(blobURL)` — the usual reason to make one —
//   constructs and never runs, the standing Worker residual (worker.js);
// - no `File` and no `FileReader`.
// Runs after encoding.js (its UTF-8 codec) and url.js (extends `URL`).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var encoder = new g.TextEncoder();
  var decoder = new g.TextDecoder();

  // A valid type is lowercased printable ASCII; anything else is no type.
  function mime(t) {
    t = String(t);
    return /^[\x20-\x7e]*$/.test(t) ? t.toLowerCase() : '';
  }
  function make(bytes, type) {
    var b = Object.create(Blob.prototype);
    Object.assign(slots(b), { bytes: bytes, type: type });
    return b;
  }
  // The receiver's bytes, or the WebIDL brand-check failure.
  function own(blob, member) {
    var bytes = slots(blob).bytes;
    if (!bytes) {
      throw new TypeError("'" + member + "' called on an object that does not implement "
        + 'interface Blob.');
    }
    return bytes;
  }
  function partBytes(part, native) {
    if (part instanceof ArrayBuffer) return new Uint8Array(part);
    if (ArrayBuffer.isView(part)) {
      return new Uint8Array(part.buffer, part.byteOffset, part.byteLength);
    }
    if (slots(part).bytes) return slots(part).bytes;
    var text = String(part);
    // `endings: "native"` on Linux: every line break in a string part is LF.
    return encoder.encode(native ? text.replace(/\r\n|\r/g, '\n') : text);
  }

  var Blob = g.__frot_iface('Blob', null, function (inst, args) {
    var parts = args[0], options = args[1], at = 'Blob constructor: ';
    if (parts !== undefined
      && (parts === null || typeof parts !== 'object' || typeof parts[Symbol.iterator] !== 'function')) {
      throw new TypeError(at + "Argument 1 can't be converted to a sequence.");
    }
    if (options !== undefined && options !== null && typeof options !== 'object') {
      throw new TypeError(at + "Argument 2 can't be converted to a dictionary.");
    }
    options = options || {};
    var endings = options.endings === undefined ? 'transparent' : String(options.endings);
    if (endings !== 'transparent' && endings !== 'native') {
      throw new TypeError(at + "'" + endings + "' (value of 'endings' member of "
        + 'BlobPropertyBag) is not a valid value for enumeration EndingType.');
    }
    var chunks = Array.from(parts || [], function (p) {
      return partBytes(p, endings === 'native');
    });
    var bytes = new Uint8Array(chunks.reduce(function (n, c) {
      return n + c.length;
    }, 0));
    chunks.reduce(function (at, c) {
      bytes.set(c, at);
      return at + c.length;
    }, 0);
    Object.assign(slots(inst), {
      bytes: bytes, type: options.type === undefined ? '' : mime(options.type),
    });
  });
  // A promise for `read(bytes)`; a wrong receiver REJECTS (measured).
  function later(member, read) {
    return function () {
      var blob = this;
      return new Promise(function (resolve) {
        resolve(read(own(blob, member)));
      });
    };
  }
  g.__frot_ifaceops(Blob.prototype, {
    slice: function slice() {
      var bytes = own(this, 'slice'), size = bytes.length;
      // A relative index: negative counts from the end; both clamp to the blob.
      function index(v, fallback) {
        var n = v === undefined ? fallback : Math.trunc(Number(v)) || 0;
        return n < 0 ? Math.max(size + n, 0) : Math.min(n, size);
      }
      var start = index(arguments[0], 0), end = index(arguments[1], size);
      var type = arguments[2] === undefined ? '' : mime(arguments[2]);
      return make(bytes.slice(start, Math.max(start, end)), type);
    },
    text: later('text', function (bytes) {
      return decoder.decode(bytes);
    }),
    arrayBuffer: later('arrayBuffer', function (bytes) {
      return bytes.slice().buffer;
    }),
    bytes: later('bytes', function (bytes) {
      return bytes.slice();
    }),
  });
  g.__frot_ifaceattrs(Blob.prototype, {
    size: function () {
      return own(this, 'get size').length;
    },
    type: function () {
      own(this, 'get type');
      return slots(this).type;
    },
  });

  // --- blob: URLs ----------------------------------------------------------------
  // `blob:<origin>/<uuid>` (`blob:null/<uuid>` on a file: page), fresh per call.
  // The id is a v4 UUID from the OS entropy source, as crypto.randomUUID's is.
  function uuid() {
    var b = g.__frot_random_bytes(16), s = '';
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    for (var i = 0; i < 16; i++) {
      s += (b[i] + 0x100).toString(16).slice(1);
      if (i === 3 || i === 5 || i === 7 || i === 9) s += '-';
    }
    return s;
  }
  function need(args, name) {
    if (!args.length) {
      throw new TypeError('URL.' + name + ': At least 1 argument required, but only 0 passed');
    }
  }
  g.__frot_ifaceops(g.URL, {
    createObjectURL: function createObjectURL(blob) {
      need(arguments, 'createObjectURL');
      if (!slots(blob).bytes) {
        throw new TypeError('URL.createObjectURL: Argument 1 could not be converted to any of: '
          + 'Blob, MediaSource.');
      }
      return 'blob:' + g.__frot_location().origin + '/' + uuid();
    },
    // Nothing holds a blob: URL's bytes (nothing can load one), so revoking has
    // nothing to release.
    revokeObjectURL: function revokeObjectURL(url) {
      need(arguments, 'revokeObjectURL');
    },
  });
})(globalThis);
