// `FileReader` over blob.js's `Blob`, and the `ProgressEvent` it fires
// (bl-8c46; shapes, event order, payloads and messages READ off Firefox
// 153.0esr, identity.md §3.20). AWS WAF's challenge base64-encodes a Blob with
// `readAsDataURL` before falling back to `btoa`.
//
// A read is one task on the virtual clock: `loadstart`, `progress` (unless the
// blob is empty), `load`, `loadend`, in that task, after the microtasks of the
// call that started it. The bytes are already in memory, so nothing can fail
// and `error` never fires. Text is decoded as UTF-8 — the one encoding
// encoding.js has; a legacy `encoding` argument or `charset` is not honoured
// (identity.md §11). Runs after blob.js, events.js and loop.js.
(function (g) {
  'use strict';
  var slots = g.__frot_slots, brand = g.__frot_brand, fail = g.__frot_domerror;
  var setTimeout = g.setTimeout; // the loop's, whatever a page later assigns
  var decoder = new g.TextDecoder();

  // --- ProgressEvent -------------------------------------------------------------
  function finite(init, key) {
    var n = init[key] === undefined ? 0 : Number(init[key]);
    if (!isFinite(n)) {
      throw new TypeError("ProgressEvent constructor: '" + key
        + "' member of ProgressEventInit is not a finite floating-point value.");
    }
    return n;
  }
  var ProgressEvent = g.__frot_iface('ProgressEvent', null, function (inst, args) {
    var init = args[1], at = 'ProgressEvent constructor: ';
    if (!args.length) throw new TypeError(at + 'At least 1 argument required, but only 0 passed');
    if (init !== undefined && init !== null && typeof init !== 'object') {
      throw new TypeError(at + "Value can't be converted to a dictionary.");
    }
    g.__frot_event_init(inst, args[0], init);
    init = init || {};
    Object.assign(slots(inst), {
      lengthComputable: !!init.lengthComputable,
      loaded: finite(init, 'loaded'), total: finite(init, 'total'),
    });
  });
  Object.setPrototypeOf(ProgressEvent.prototype, g.Event.prototype);
  g.__frot_ifaceattrs(ProgressEvent.prototype, ['lengthComputable', 'loaded', 'total']);
  Object.defineProperty(ProgressEvent, 'length', { value: 1, configurable: true });

  // --- FileReader ----------------------------------------------------------------
  var STATES = { EMPTY: 0, LOADING: 1, DONE: 2 };
  var TYPES = ['loadstart', 'progress', 'load', 'abort', 'error', 'loadend'];
  var FileReader = g.__frot_iface('FileReader', null, function (inst) {
    Object.assign(slots(inst), { readyState: 0, result: null, error: null, run: 0, on: {} });
  });
  var proto = FileReader.prototype;
  Object.setPrototypeOf(proto, g.EventTarget.prototype);
  // The receiver's state, or the WebIDL brand-check failure.
  function own(reader, member) {
    var st = slots(reader);
    if (!st.on) {
      throw new TypeError("'" + member + "' called on an object that does not implement "
        + 'interface FileReader.');
    }
    return st;
  }
  function emit(reader, type, loaded, total) {
    var ev = new ProgressEvent(type, { lengthComputable: true, loaded: loaded, total: total });
    slots(ev).trusted = true; // the browser raised it, not the page
    reader.dispatchEvent(ev);
  }
  function binary(bytes) {
    var s = '';
    for (var i = 0; i < bytes.length; i += 8192) {
      s += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
    }
    return s;
  }
  // Each reader: (bytes, the blob's type) -> `result`.
  var READS = {
    readAsArrayBuffer: function (bytes) {
      return bytes.slice().buffer;
    },
    readAsBinaryString: binary,
    readAsText: function (bytes) {
      return decoder.decode(bytes);
    },
    readAsDataURL: function (bytes, type) {
      return 'data:' + (type || 'application/octet-stream') + ';base64,' + g.btoa(binary(bytes));
    },
  };
  var ops = {};
  Object.keys(READS).forEach(function (name) {
    ops[name] = function (blob) {
      var reader = this, st = own(reader, name), at = 'FileReader.' + name + ': ';
      if (!arguments.length) {
        throw new TypeError(at + 'At least 1 argument required, but only 0 passed');
      }
      if (blob === null || typeof blob !== 'object') {
        throw new TypeError(at + 'Argument 1 is not an object.');
      }
      var bytes = slots(blob).bytes;
      if (!bytes) throw new TypeError(at + 'Argument 1 does not implement interface Blob.');
      if (st.readyState === STATES.LOADING) {
        throw fail('InvalidStateError',
          'An attempt was made to use an object that is not, or is no longer, usable');
      }
      var run = ++st.run, size = bytes.length;
      Object.assign(st, { readyState: STATES.LOADING, result: null, error: null, size: size });
      // `abort()` or a new read moves `run` on, and this one goes quiet — until
      // `load`, after which `loadend` follows whatever the handler did (measured).
      setTimeout(function () {
        if (st.run === run) emit(reader, 'loadstart', 0, size);
        if (st.run === run && size) emit(reader, 'progress', size, size);
        if (st.run !== run) return;
        st.readyState = STATES.DONE;
        st.result = READS[name](bytes, slots(blob).type);
        emit(reader, 'load', size, size);
        emit(reader, 'loadend', size, size);
      }, 0);
    };
    Object.defineProperty(ops[name], 'length', { value: 1, configurable: true });
  });
  // Only a read in flight can be aborted; its two events are synchronous and
  // carry the blob's size as `total` (measured). The message's trailing space
  // is Gecko's.
  ops.abort = function () {
    var st = own(this, 'abort');
    if (st.readyState !== STATES.LOADING) return;
    st.run++;
    Object.assign(st, {
      readyState: STATES.DONE, result: null,
      error: fail('AbortError', 'The operation was aborted. '),
    });
    emit(this, 'abort', 0, st.size);
    emit(this, 'loadend', 0, st.size);
  };
  g.__frot_ifaceops(proto, ops);
  var attrs = {};
  ['readyState', 'result', 'error'].forEach(function (key) {
    attrs[key] = function () {
      return own(this, 'get ' + key)[key];
    };
  });
  g.__frot_ifaceattrs(proto, attrs);
  // An event-handler attribute is a listener like any other, taking its place
  // in the order when it is first set and losing it when set to null (measured:
  // listener, handler, listener). Anything but a function reads back null.
  TYPES.forEach(function (type) {
    var key = 'on' + type;
    Object.defineProperty(proto, key, {
      get: brand(function () {
        var held = own(this, 'get ' + key).on[type];
        return held ? held.fn : null;
      }, 'get ' + key),
      set: brand(function (fn) {
        var on = own(this, 'set ' + key).on, held = on[type];
        if (typeof fn !== 'function') {
          if (held) this.removeEventListener(type, held.call);
          delete on[type];
        } else if (held) {
          held.fn = fn;
        } else {
          held = on[type] = {
            fn: fn,
            call: function (ev) {
              held.fn.call(this, ev);
            },
          };
          this.addEventListener(type, held.call);
        }
      }, 'set ' + key),
      enumerable: true,
      configurable: true,
    });
  });
  [FileReader, proto].forEach(function (o) {
    Object.keys(STATES).forEach(function (name) {
      Object.defineProperty(o, name, { value: STATES[name], enumerable: true });
    });
  });
})(globalThis);
