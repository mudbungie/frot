// FormData (bl-6eb9, challenge.md §11.4) — the request body AWS WAF's challenge
// builds for its token POST: `new FormData()`, string fields appended, handed
// to `fetch`. A real entry list, and the real `multipart/form-data` bytes
// Firefox puts on the wire for it. Member order, `.length`s, error messages,
// the boundary format, and the name/value escaping were all READ off Firefox
// 153.0esr (identity.md §3.20).
//
// Declared short of Firefox (identity.md §11): `new FormData(form)` — collecting
// a <form>'s controls — is refused, since frot originates no form submission
// (challenge.md §2); a `Blob` value is kept as that Blob (it becomes a `File`
// on the binary) and a FormData holding one cannot be POSTed, because the
// request path carries text only (challenge.md §3); and the iterators are array
// iterators, not `FormData Iterator`s. Runs after blob.js; net.js takes the
// serializer.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var Blob = g.Blob;

  function need(count, args, name) {
    if (args.length < count) {
      throw new TypeError('FormData.' + name + ': At least ' + count
        + ' argument required, but only ' + args.length + ' passed');
    }
  }
  // append/set are overloaded (value, or blob + filename), so WebIDL words a
  // short call differently.
  function overload(args, name) {
    if (args.length < 2) {
      throw new TypeError('FormData.' + name + ': ' + args.length
        + ' is not a valid argument count for any overload.');
    }
  }
  // An entry's value: a Blob as itself, anything else as its string. The
  // three-argument form names a file, so its value must be a Blob.
  function entryValue(args, name) {
    var value = args[1];
    if (value instanceof Blob) return value;
    if (args.length > 2) throw new TypeError('FormData.' + name + ': Argument 2 is not an object.');
    return String(value);
  }
  function list(fd, name) {
    var l = slots(fd).list;
    if (!l) {
      throw new TypeError("'" + name + "' called on an object that does not implement "
        + 'interface FormData.');
    }
    return l;
  }
  // Every value filed under `name`, in order.
  function named(fd, method, name) {
    name = String(name);
    return list(fd, method).filter(function (e) {
      return e[0] === name;
    }).map(function (e) {
      return e[1];
    });
  }
  function column(fd, name, pick) {
    return list(fd, name).map(pick)[Symbol.iterator]();
  }

  var FormData = g.__frot_iface('FormData', null, function (inst, args) {
    var form = args[0];
    if (form !== undefined) {
      if (form === null || typeof form !== 'object') {
        throw new TypeError('FormData constructor: Argument 1 is not an object.');
      }
      if (form.nodeType !== 1 || form.tagName !== 'FORM') {
        throw new TypeError('FormData constructor: Argument 1 does not implement interface '
          + 'HTMLFormElement.');
      }
      throw new TypeError('frot: FormData from a form element is not supported');
    }
    slots(inst).list = [];
  });
  var proto = FormData.prototype;
  g.__frot_ifaceops(proto, {
    append: function append(name, value) {
      overload(arguments, 'append');
      list(this, 'append').push([String(name), entryValue(arguments, 'append')]);
    },
    'delete': function (name) {
      need(1, arguments, 'delete');
      name = String(name);
      slots(this).list = list(this, 'delete').filter(function (e) {
        return e[0] !== name;
      });
    },
    get: function get(name) {
      need(1, arguments, 'get');
      var all = named(this, 'get', name);
      return all.length ? all[0] : null;
    },
    getAll: function getAll(name) {
      need(1, arguments, 'getAll');
      return named(this, 'getAll', name);
    },
    has: function has(name) {
      need(1, arguments, 'has');
      return named(this, 'has', name).length > 0;
    },
    // Replace the first entry of that name in place, drop the rest; else append.
    set: function set(name, value) {
      overload(arguments, 'set');
      name = String(name);
      var entry = [name, entryValue(arguments, 'set')], placed = false;
      var kept = list(this, 'set').filter(function (e) {
        return e[0] !== name || (!placed && (placed = true));
      }).map(function (e) {
        return e[0] === name ? entry : e;
      });
      if (!placed) kept.push(entry);
      slots(this).list = kept;
    },
    entries: function entries() {
      return column(this, 'entries', function (e) {
        return [e[0], e[1]];
      });
    },
    keys: function keys() {
      return column(this, 'keys', function (e) {
        return e[0];
      });
    },
    values: function values() {
      return column(this, 'values', function (e) {
        return e[1];
      });
    },
    forEach: function forEach(callback) {
      if (typeof callback !== 'function') {
        throw new TypeError('FormData.forEach: Argument 1 is not an object.');
      }
      var thisArg = arguments[1];
      list(this, 'forEach').slice().forEach(function (e) {
        callback.call(thisArg, e[1], e[0], this);
      }, this);
    },
  });
  Object.defineProperty(proto, Symbol.iterator, {
    value: proto.entries, writable: true, configurable: true,
  });

  // --- multipart/form-data, as Firefox serializes it ----------------------------
  // Every line break in a name or value becomes CRLF; then, in a NAME, CR, LF
  // and the double quote are percent-escaped. Text is sent as UTF-8, unescaped.
  function crlf(s) {
    return s.replace(/\r\n|\r|\n/g, '\r\n');
  }
  function fieldName(s) {
    return crlf(s).replace(/\r/g, '%0D').replace(/\n/g, '%0A').replace(/"/g, '%22');
  }
  // `----geckoformboundary` + two random 64-bit numbers in hex. Firefox prints
  // them unpadded, so the tail is 32 hex digits, or 31 about one time in six
  // (measured over 40 boundaries) — reproduced by dropping each half's leading
  // zeros. Real entropy, the getRandomValues source: a boundary must not be
  // predictable from the body it delimits.
  function boundary() {
    var r = g.__frot_random_bytes(16), out = '----geckoformboundary';
    [0, 8].forEach(function (at) {
      var half = '';
      for (var i = at; i < at + 8; i++) half += (r[i] + 256).toString(16).slice(1);
      out += half.replace(/^0+(?=.)/, '');
    });
    return out;
  }
  // `{ body, type }` for a FormData, `{ error }` for one holding bytes (a Blob),
  // or null for anything else.
  function multipart(fd) {
    var entries = slots(fd).list;
    if (!entries) return null;
    var bytes = entries.some(function (e) {
      return typeof e[1] !== 'string';
    });
    if (bytes) return { error: 'frot: only text request bodies are supported' };
    var b = boundary();
    var parts = entries.map(function (e) {
      return '--' + b + '\r\nContent-Disposition: form-data; name="' + fieldName(e[0])
        + '"\r\n\r\n' + crlf(e[1]) + '\r\n';
    });
    return { body: parts.join('') + '--' + b + '--\r\n', type: 'multipart/form-data; boundary=' + b };
  }
  Object.defineProperty(g, '__frot_multipart', { value: multipart, configurable: true });
})(globalThis);
