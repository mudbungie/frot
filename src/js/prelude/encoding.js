// TextEncoder / TextDecoder (bl-53b7) — UTF-8, both directions, in pure JS: the
// encoder and the WHATWG UTF-8 decoder are deterministic functions of their
// input, so they need no syscall. AWS WAF's challenge feature-tests both
// (challenge.md §11.4). Member order, `.length`s, the label list, the
// replacement-character behaviour on malformed input, BOM handling across
// streamed chunks, and every error message were READ off Firefox 153.0esr
// (identity.md §3.20).
//
// Declared short of Firefox (identity.md §11): UTF-8 only. Firefox decodes ~40
// legacy encodings; here any other label is refused with the RangeError Firefox
// gives an UNKNOWN label — a refusal, never mis-decoded text. Runs after
// brand.js.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var iface = g.__frot_iface;
  var ops = g.__frot_ifaceops;
  var attrs = g.__frot_ifaceattrs;

  // A string's UTF-8 bytes, as a plain array. A lone surrogate is U+FFFD — a
  // USVString conversion, as measured. `stop(units, size)` may end the walk
  // early (encodeInto's "does the next scalar still fit"); `units` read so far
  // is left in `done.read`.
  function utf8(str, stop, done) {
    var out = [];
    for (var i = 0; i < str.length; i++) {
      var c = str.charCodeAt(i), units = 1;
      if (c >= 0xd800 && c <= 0xdbff && i + 1 < str.length) {
        var d = str.charCodeAt(i + 1);
        if (d >= 0xdc00 && d <= 0xdfff) {
          c = 0x10000 + ((c - 0xd800) << 10) + (d - 0xdc00);
          units = 2;
        }
      }
      if (c >= 0xd800 && c <= 0xdfff) c = 0xfffd;
      var size = c < 0x80 ? 1 : c < 0x800 ? 2 : c < 0x10000 ? 3 : 4;
      if (stop && stop(out.length + size)) break;
      if (size === 1) out.push(c);
      else if (size === 2) out.push(0xc0 | (c >> 6), 0x80 | (c & 63));
      else if (size === 3) out.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
      else {
        out.push(0xf0 | (c >> 18), 0x80 | ((c >> 12) & 63), 0x80 | ((c >> 6) & 63),
          0x80 | (c & 63));
      }
      i += units - 1;
      if (done) done.read = i + 1;
    }
    return out;
  }

  var TextEncoder = iface('TextEncoder', null, function () {});
  ops(TextEncoder.prototype, {
    encode: function encode() {
      var input = arguments[0];
      return Uint8Array.from(utf8(input === undefined ? '' : String(input)));
    },
    encodeInto: function encodeInto(source, destination) {
      if (!(destination instanceof Uint8Array)) {
        throw new TypeError('TextEncoder.encodeInto: Argument 2 does not implement interface '
          + 'Uint8Array.');
      }
      var done = { read: 0 };
      var bytes = utf8(String(source), function (size) {
        return size > destination.length;
      }, done);
      destination.set(bytes);
      return { read: done.read, written: bytes.length };
    },
  });
  attrs(TextEncoder.prototype, {
    encoding: function () {
      return 'utf-8';
    },
  });

  // The labels that mean UTF-8 (the WHATWG list; each one measured).
  var UTF8_LABELS = ['unicode-1-1-utf-8', 'unicode11utf8', 'unicode20utf8', 'utf-8', 'utf8',
    'x-unicode20utf8'];

  var TextDecoder = iface('TextDecoder', null, function (inst, args) {
    var label = args[0] === undefined ? 'utf-8' : String(args[0]);
    if (UTF8_LABELS.indexOf(label.replace(/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, '').toLowerCase()) < 0) {
      throw new RangeError("TextDecoder constructor: The given encoding '" + label
        + "' is not supported.");
    }
    var options = args[1] || {};
    Object.assign(slots(inst), {
      encoding: 'utf-8', fatal: !!options.fatal, ignoreBOM: !!options.ignoreBOM,
      state: fresh(),
    });
  });
  // The WHATWG UTF-8 decoder's state, carried between `{stream: true}` calls:
  // the code point under construction, continuation bytes still needed, the
  // allowed range of the next one, and whether a leading BOM is still possible.
  function fresh() {
    return { cp: 0, need: 0, lo: 0x80, hi: 0xbf, start: true };
  }
  function decodeBytes(st, bytes, stream) {
    var s = st.state, text = '', units = [];
    function emit(cp) {
      var bom = s.start && cp === 0xfeff && !st.ignoreBOM;
      s.start = false;
      if (bom) return;
      if (cp > 0xffff) units.push(0xd800 + ((cp - 0x10000) >> 10), 0xdc00 + ((cp - 0x10000) & 1023));
      else units.push(cp);
      if (units.length >= 8192) flush();
    }
    function flush() {
      text += String.fromCharCode.apply(null, units);
      units = [];
    }
    function bad() {
      if (st.fatal) {
        st.state = fresh();
        throw new TypeError('TextDecoder.decode: Decoding failed.');
      }
      emit(0xfffd);
    }
    for (var i = 0; i < bytes.length; i++) {
      var b = bytes[i];
      if (s.need === 0) {
        if (b < 0x80) emit(b);
        else if (b >= 0xc2 && b <= 0xdf) (s.need = 1, s.cp = b & 0x1f);
        else if (b >= 0xe0 && b <= 0xef) {
          if (b === 0xe0) s.lo = 0xa0;
          if (b === 0xed) s.hi = 0x9f;
          (s.need = 2, s.cp = b & 0xf);
        } else if (b >= 0xf0 && b <= 0xf4) {
          if (b === 0xf0) s.lo = 0x90;
          if (b === 0xf4) s.hi = 0x8f;
          (s.need = 3, s.cp = b & 0x7);
        } else bad();
      } else if (b < s.lo || b > s.hi) {
        // Not a continuation: the sequence so far is one error, and this byte
        // is read again as the start of the next.
        (s.need = 0, s.lo = 0x80, s.hi = 0xbf);
        bad();
        i--;
      } else {
        (s.lo = 0x80, s.hi = 0xbf);
        s.cp = (s.cp << 6) | (b & 0x3f);
        if (--s.need === 0) emit(s.cp);
      }
    }
    if (!stream) {
      var cut = s.need !== 0;
      st.state = s = fresh();
      if (cut) {
        s.start = false;
        bad();
        st.state = fresh();
      }
    }
    flush();
    return text;
  }
  ops(TextDecoder.prototype, {
    decode: function decode() {
      var input = arguments[0], options = arguments[1] || {};
      var bytes;
      if (input === undefined) bytes = [];
      else if (input instanceof ArrayBuffer) bytes = new Uint8Array(input);
      else if (ArrayBuffer.isView(input)) {
        bytes = new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
      } else {
        throw new TypeError('TextDecoder.decode: Argument 1 could not be converted to any of: '
          + 'ArrayBufferView, ArrayBuffer.');
      }
      return decodeBytes(slots(this), bytes, !!options.stream);
    },
  });
  attrs(TextDecoder.prototype, ['encoding', 'fatal', 'ignoreBOM']);
})(globalThis);
