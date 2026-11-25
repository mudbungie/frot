// Streams, part 6 (bl-96f7): TransformStream and its controller, the two
// queuing strategies, TextEncoderStream/TextDecoderStream (over encoding.js,
// so UTF-8 only — its stated residual), and `Blob.prototype.stream`.
(function (g) {
  'use strict';
  var slots = g.__frot_slots, S = g.__frot_streams;

  function rc(ts) {
    return slots(slots(ts).readable).controller;
  }
  function setBackpressure(ts, bp) {
    var st = slots(ts);
    if (st.bpChange) st.bpChange.resolve();
    st.bpChange = S.defer();
    st.backpressure = bp;
  }
  function clear(ts) {
    var cs = slots(slots(ts).controller);
    cs.transform = cs.flush = undefined;
  }
  function unblock(ts) {
    if (slots(ts).backpressure) setBackpressure(ts, false);
  }
  function errorWritable(ts, e) {
    clear(ts);
    S.wsErrorIfNeeded(slots(slots(ts).writable).controller, e);
    unblock(ts);
  }
  function error(ts, e) {
    S.dcError(rc(ts), e);
    errorWritable(ts, e);
  }
  function enqueue(ts, chunk) {
    var c = rc(ts);
    if (!S.canEnqueue(c)) {
      throw new TypeError('TransformStreamDefaultController.enqueue: Readable side is not in a '
        + 'state that permits enqueue');
    }
    try {
      S.dcEnqueue(c, chunk);
    } catch (e) {
      errorWritable(ts, e);
      throw slots(slots(ts).readable).storedError;
    }
    if (!S.shouldPull(c) !== slots(ts).backpressure) setBackpressure(ts, true);
  }
  function perform(ts, chunk) {
    return slots(slots(ts).controller).transform(chunk).then(undefined, function (r) {
      error(ts, r);
      throw r;
    });
  }
  // The writable side's algorithms. Firefox 153 implements the standard as it
  // stood before transformers had a `cancel` (measured: `cancel` is never
  // called, and a cancelled readable errors the writable with its reason), so
  // abort and a readable-side cancel just error the other side.
  function sink(ts) {
    return [function (chunk) {
      var st = slots(ts);
      if (!st.backpressure) return perform(ts, chunk);
      return st.bpChange.promise.then(function () {
        var ws = slots(st.writable);
        if (ws.state === 'erroring') throw ws.storedError;
        return perform(ts, chunk);
      });
    }, function () {
      var rs = slots(slots(ts).readable), p = slots(slots(ts).controller).flush();
      clear(ts);
      return p.then(function () {
        if (rs.state === 'errored') throw rs.storedError;
        S.dcClose(rc(ts));
      }, function (r) {
        error(ts, r);
        throw rs.storedError;
      });
    }, function (reason) {
      error(ts, reason);
      return Promise.resolve();
    }];
  }

  var Controller = g.__frot_iface('TransformStreamDefaultController');
  var at = 'TransformStream constructor: ', dat = 'TransformStream.constructor: ';
  var TS = S.ctor('TransformStream', 0, function (inst, args) {
    var tr = args[0];
    if (tr !== undefined && !S.isObj(tr)) throw new TypeError(at + 'Argument 1 is not an object.');
    var wstrat = S.strategy(args[1], at, 2), rstrat = S.strategy(args[2], at, 3);
    var d = tr === undefined ? {} : tr;
    function cb(k) {
      return S.callback(d, k, dat, 'Transformer');
    }
    // Transformer's members, in WebIDL's conversion order.
    var flush = cb('flush'), rtype = d.readableType, start = cb('start');
    var transform = cb('transform'), wtype = d.writableType;
    [['readableType', rtype], ['writableType', wtype]].forEach(function (p) {
      if (p[1] !== undefined) {
        throw new RangeError(at + '`' + p[0] + '` is unsupported and preserved for future use');
      }
    });
    var rh = S.mark(rstrat, 0, at), wh = S.mark(wstrat, 1, at), started = S.defer();
    var st = Object.assign(slots(inst), { tag: 'TS', backpressure: undefined,
      bpChange: undefined });
    var algos = sink(inst);
    function begin() {
      return started.promise;
    }
    st.writable = S.wsCreate(begin, algos[0], algos[1], algos[2], wh, S.sizer(wstrat));
    st.readable = S.create(begin, function () {
      setBackpressure(inst, false);
      return slots(inst).bpChange.promise;
    }, function (reason) {
      errorWritable(inst, reason);
      return Promise.resolve();
    }, rh, S.sizer(rstrat));
    setBackpressure(inst, true);
    var c = Object.create(Controller.prototype);
    st.controller = c;
    Object.assign(slots(c), { tag: 'TSDC', stream: inst,
      transform: transform ? function (chunk) {
        return S.pcall(transform, tr, [chunk, c]);
      } : function (chunk) {
        try {
          enqueue(inst, chunk);
          return Promise.resolve();
        } catch (e) {
          return Promise.reject(e);
        }
      },
      flush: function () {
        return S.pcall(flush, tr, [c]);
      } });
    started.resolve(start ? start.call(tr, c) : undefined);
  });
  S.members(Controller.prototype, 'TSDC', 'TransformStreamDefaultController', {
    enqueue: function enqueue_() {
      enqueue(slots(this).stream, arguments[0]);
    },
    error: function error_() {
      error(slots(this).stream, arguments[0]);
    },
    terminate: function terminate() {
      var ts = slots(this).stream;
      S.dcClose(rc(ts));
      errorWritable(ts, new TypeError('Terminating the stream'));
    },
    get_desiredSize: function () {
      return S.desired(rc(slots(this).stream));
    },
  });
  // The two ends, shared by every transform-shaped interface.
  var ends = {
    get_readable: function () {
      return slots(this).readable;
    },
    get_writable: function () {
      return slots(this).writable;
    },
  };
  S.members(TS.prototype, 'TS', 'TransformStream', ends);

  // --- Queuing strategies: one shared `size` function per interface ----------
  function strategyIface(name, size) {
    var C = S.ctor(name, 1, function (inst, args) {
      var w = name + ' constructor: ';
      S.need(args, 1, w);
      var hwm = S.dict(args[0], w, 1).highWaterMark;
      if (hwm === undefined) {
        throw new TypeError(w + "Missing required 'highWaterMark' member of QueuingStrategyInit.");
      }
      Object.assign(slots(inst), { tag: name, hwm: Number(hwm) });
    });
    g.__frot_brand(size, 'size');
    S.members(C.prototype, name, name, {
      get_highWaterMark: function () {
        return slots(this).hwm;
      },
      get_size: function () {
        return size;
      },
    });
  }
  strategyIface('ByteLengthQueuingStrategy', function (chunk) {
    return chunk.byteLength;
  });
  strategyIface('CountQueuingStrategy', function () {
    return 1;
  });

  // --- Text streams: a TransformStream around encoding.js's codecs ------------
  function textStream(name, make, attrs) {
    var C = S.ctor(name, 0, function (inst, args) {
      var codec = make(args), ts = new TS(codec.transformer);
      Object.assign(slots(inst), { tag: name, codec: codec.state,
        readable: slots(ts).readable, writable: slots(ts).writable });
    });
    var table = Object.assign({}, ends);
    attrs.forEach(function (k) {
      table['get_' + k] = function () {
        return slots(this).codec[k];
      };
    });
    S.members(C.prototype, name, name, table);
  }
  textStream('TextEncoderStream', function () {
    var enc = new g.TextEncoder(), held = '';
    return { state: enc, transformer: {
      transform: function (chunk, c) {
        var s = held + String(chunk), last = s.charCodeAt(s.length - 1);
        held = last >= 0xd800 && last <= 0xdbff ? s.slice(-1) : '';
        if (held) s = s.slice(0, -1);
        if (s) c.enqueue(enc.encode(s));
      },
      flush: function (c) {
        if (held) c.enqueue(Uint8Array.of(0xef, 0xbf, 0xbd));
      },
    } };
  }, ['encoding']);
  textStream('TextDecoderStream', function (args) {
    var dec;
    try {
      dec = new g.TextDecoder(args[0], args[1]);
    } catch (e) {
      e.message = e.message.replace('TextDecoder constructor', 'TextDecoderStream constructor');
      throw e;
    }
    function out(c, s) {
      if (s) c.enqueue(s);
    }
    return { state: dec, transformer: {
      transform: function (chunk, c) {
        if (!(chunk instanceof ArrayBuffer) && !ArrayBuffer.isView(chunk)) {
          throw new TypeError('TextDecoderStream: chunk could not be converted to any of: '
            + 'ArrayBufferView, ArrayBuffer.');
        }
        out(c, dec.decode(chunk, { stream: true }));
      },
      flush: function (c) {
        out(c, dec.decode());
      },
    } };
  }, ['encoding', 'fatal', 'ignoreBOM']);

  // --- A byte stream of whole bytes: Blob.stream() and Response.body ---------
  function bytesStream(bytes) {
    return S.createBytes(function (c) {
      if (bytes.length) S.byteEnqueue(c, bytes.slice());
      S.byteClose(c);
    });
  }
  var BP = g.Blob.prototype, after = Object.getOwnPropertyNames(BP).slice(1);
  var moved = after.map(function (k) {
    var d = Object.getOwnPropertyDescriptor(BP, k);
    delete BP[k];
    return [k, d];
  });
  g.__frot_ifaceops(BP, { stream: function stream() {
    var b = slots(this).bytes;
    if (!b) {
      throw new TypeError("'stream' called on an object that does not implement interface Blob.");
    }
    return bytesStream(b);
  } });
  moved.forEach(function (p) {
    Object.defineProperty(BP, p[0], p[1]);
  });
  S.bytesStream = bytesStream;
})(globalThis);
