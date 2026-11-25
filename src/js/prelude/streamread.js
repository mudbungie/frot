// Streams, part 2 (bl-96f7): the ReadableStream interface itself, its default
// controller's members, and both readers. The abstract operations are
// streams.js's; the byte controller (streambyte.js) supplies `setupBytes` and
// `pullInto`, looked up when a byte stream is first made. Here too: the helpers
// every stream file shapes its members with (`members`, `ctor`, `need`).
(function (g) {
  'use strict';
  var slots = g.__frot_slots, S = g.__frot_streams, brand = g.__frot_brand;
  var iface = g.__frot_iface;
  var at = 'ReadableStream constructor: ', dat = 'ReadableStream.constructor: ';

  // Wrap a table of members so a foreign receiver fails the WebIDL brand check
  // (`'read' called on an object that does not implement interface …`) — thrown,
  // or a rejection for the promise-returning ones. `.length` is the member's.
  function guard(tag, name, table, async) {
    var out = {};
    Object.keys(table).forEach(function (k) {
      var f = table[k];
      var key = k.replace(/^get_/, 'get ');
      out[k] = function () {
        if (slots(this).tag === tag) return f.apply(this, arguments);
        var e = new TypeError("'" + key + "' called on an object that does not implement interface "
          + name + '.');
        if (async && async.indexOf(k) >= 0) return Promise.reject(e);
        throw e;
      };
      Object.defineProperty(out[k], 'length', { value: f.length, configurable: true });
    });
    return out;
  }
  // Members onto a prototype: methods (`name`) and getters (`get_name`), in the
  // given order, each branded native.
  function members(proto, tag, name, table, async) {
    var w = guard(tag, name, table, async);
    Object.keys(w).forEach(function (k) {
      if (k.indexOf('get_') === 0) {
        var key = k.slice(4);
        Object.defineProperty(proto, key, { get: brand(w[k], 'get ' + key),
          enumerable: true, configurable: true });
      } else {
        proto[k] = brand(w[k], k);
      }
    });
  }
  // An interface a page may construct, `.length` as measured.
  function ctor(name, len, body) {
    var C = iface(name, null, body);
    Object.defineProperty(C, 'length', { value: len, configurable: true });
    return C;
  }
  function need(args, n, at) {
    if (args.length < n) {
      throw new TypeError(at + 'At least ' + n + ' argument' + (n > 1 ? 's' : '')
        + ' required, but only ' + args.length + ' passed');
    }
  }

  Object.assign(S, { members: members, ctor: ctor, need: need });

  var ReadableStream = S.ctor('ReadableStream', 0, function (inst, args) {
    var src = args[0];
    if (src !== undefined && !S.isObj(src)) throw new TypeError(at + 'Argument 1 is not an object.');
    var strat = S.strategy(args[1], at, 2);
    var d = src === undefined ? {} : src;
    // UnderlyingSource's members, in WebIDL's (alphabetical) conversion order.
    var auto = d.autoAllocateChunkSize;
    if (auto !== undefined) auto = Number(auto);
    var cancel = S.callback(d, 'cancel', dat, 'UnderlyingSource');
    var pull = S.callback(d, 'pull', dat, 'UnderlyingSource');
    var start = S.callback(d, 'start', dat, 'UnderlyingSource');
    var type = d.type === undefined ? undefined : String(d.type);
    if (type !== undefined && type !== 'bytes') {
      throw new TypeError(dat + "'" + type + "' (value of 'type' member of UnderlyingSource) is "
        + 'not a valid value for enumeration ReadableStreamType.');
    }
    S.init(inst);
    var algos = [function (c) {
      return start ? start.call(src, c) : undefined;
    }, function (c) {
      return S.pcall(pull, src, [c]);
    }, function (reason) {
      return S.pcall(cancel, src, [reason]);
    }];
    if (type === 'bytes') {
      if (strat.size) throw new RangeError(at + "Implementation preserved member 'size'");
      if (auto === 0) throw new TypeError(at + 'autoAllocateChunkSize can not be zero.');
      S.setupBytes(inst, algos[0], algos[1], algos[2], S.mark(strat, 0, at), auto);
    } else {
      S.setupDefault(inst, algos[0], algos[1], algos[2], S.mark(strat, 1, at), S.sizer(strat));
    }
  });

  // --- Readers ---------------------------------------------------------------
  function readerInit(reader, stream, byob) {
    var ss = slots(stream);
    var rs = Object.assign(slots(reader), { tag: byob ? 'BYOB' : 'RSDR', stream: stream,
      byob: byob, requests: [], closed: S.defer() });
    ss.reader = reader;
    if (ss.state === 'closed') rs.closed.resolve();
    if (ss.state === 'errored') {
      rs.closed.reject(ss.storedError);
      S.handle(rs.closed.promise);
    }
    return reader;
  }
  function release(reader) {
    var rs = slots(reader), ss = slots(rs.stream);
    if (ss.state !== 'readable') rs.closed = S.defer();
    rs.closed.reject(new TypeError('Releasing lock on readable stream'));
    S.handle(rs.closed.promise);
    var c = ss.controller;
    slots(c).releaseSteps(c);
    ss.reader = undefined;
    rs.stream = undefined;
    var e = new TypeError('Releasing lock');
    rs.requests.splice(0).forEach(function (q) {
      q.error(e);
    });
  }
  // A read request settling `d` with the iterator-result shape Firefox gives
  // (`done` first). A BYOB read ends with the (empty) view; a default one
  // with undefined.
  function request(d, byob) {
    return { chunk: function (v) {
      d.resolve({ done: false, value: v });
    }, close: function (v) {
      d.resolve({ done: true, value: byob ? v : undefined });
    }, error: d.reject };
  }
  function read(reader, req) {
    var ss = slots(slots(reader).stream);
    ss.disturbed = true;
    if (ss.state === 'closed') req.close();
    else if (ss.state === 'errored') req.error(ss.storedError);
    else slots(ss.controller).pullSteps(ss.controller, req);
  }
  function lockCheck(stream, at) {
    if (!(stream instanceof ReadableStream) || slots(stream).tag !== 'RS') {
      throw new TypeError(at + 'Argument 1 does not implement interface ReadableStream.');
    }
    if (S.locked(stream)) {
      throw new TypeError(at + 'Cannot create a new reader for a readable stream already locked '
        + 'by another reader.');
    }
  }
  function readerOps(name, extra) {
    var gone = function (what) {
      return Promise.reject(new TypeError(name + '.' + what + ': Reading is not possible after '
        + 'calling releaseLock.'));
    };
    return Object.assign(extra(gone), {
      releaseLock: function releaseLock() {
        if (slots(this).stream) release(this);
      },
      cancel: function cancel() {
        if (!slots(this).stream) return gone('cancel');
        return S.streamCancel(slots(this).stream, arguments[0]);
      },
      get_closed: function () {
        return slots(this).closed.promise;
      },
    });
  }

  var Reader = S.ctor('ReadableStreamDefaultReader', 1, function (inst, args) {
    var w = 'ReadableStreamDefaultReader constructor: ';
    S.need(args, 1, w);
    lockCheck(args[0], w);
    readerInit(inst, args[0], false);
  });
  S.members(Reader.prototype, 'RSDR', 'ReadableStreamDefaultReader', readerOps(
    'ReadableStreamDefaultReader', function (gone) {
      return { read: function read_() {
        if (!slots(this).stream) return gone('read');
        var d = S.defer();
        read(this, request(d));
        return d.promise;
      } };
    }), ['read', 'cancel']);

  var BYOB = S.ctor('ReadableStreamBYOBReader', 1, function (inst, args) {
    var w = 'ReadableStreamBYOBReader constructor: ';
    S.need(args, 1, w);
    lockCheck(args[0], w);
    if (!slots(slots(args[0]).controller).bytes) {
      throw new TypeError(w + 'Trying to read with incompatible controller');
    }
    readerInit(inst, args[0], true);
  });
  S.members(BYOB.prototype, 'BYOB', 'ReadableStreamBYOBReader', readerOps(
    'ReadableStreamBYOBReader', function (gone) {
      return { read: function read_(view) {
        var w = 'ReadableStreamBYOBReader.read: ';
        try {
          S.need(arguments, 1, w);
          if (!ArrayBuffer.isView(view)) {
            throw new TypeError(w + 'Argument 1 could not be converted to any of: ArrayBufferView.');
          }
          if (!view.byteLength) throw new TypeError(w + 'Zero Length View');
          if (!view.buffer.byteLength) throw new TypeError(w + 'Zero Length Buffer');
          var min = S.dict(arguments[1], w, 2).min, opt = " 'min' member of "
            + 'ReadableStreamBYOBReaderReadOptions.';
          min = min === undefined ? 1 : Number(min);
          if (!min) throw new TypeError(w + 'Zero is not a valid value for' + opt);
          var count = view instanceof DataView ? view.byteLength : view.length;
          if (min > count) throw new RangeError(w + 'Array length exceeded by' + opt);
        } catch (e) {
          return Promise.reject(e);
        }
        if (!slots(this).stream) return gone('read');
        var d = S.defer(), ss = slots(slots(this).stream);
        ss.disturbed = true;
        if (ss.state === 'errored') d.reject(ss.storedError);
        else S.pullInto(ss.controller, view, min, request(d, true));
        return d.promise;
      } };
    }), ['read', 'cancel']);

  // --- The default controller's members --------------------------------------
  var rsdc = 'ReadableStreamDefaultController.';
  S.members(S.RSDC.prototype, 'RSDC', 'ReadableStreamDefaultController', {
    close: function close() {
      if (!S.canEnqueue(this)) {
        throw new TypeError(rsdc + 'close: Cannot close a stream that is already closed.');
      }
      S.dcClose(this);
    },
    enqueue: function enqueue() {
      if (!S.canEnqueue(this)) {
        throw new TypeError(rsdc + 'enqueue: Cannot enqueue into a stream that is already closed.');
      }
      S.dcEnqueue(this, arguments[0]);
    },
    error: function error() {
      S.dcError(this, arguments[0]);
    },
    get_desiredSize: function () {
      return S.desired(this);
    },
  });

  // --- The stream's members (tee/pipe/iteration are streampipe.js's) ---------
  S.members(ReadableStream.prototype, 'RS', 'ReadableStream', {
    cancel: function cancel() {
      if (S.locked(this)) {
        return Promise.reject(new TypeError('ReadableStream.cancel: Cannot cancel a stream locked '
          + 'by a reader.'));
      }
      return S.streamCancel(this, arguments[0]);
    },
    getReader: function getReader() {
      var w = 'ReadableStream.getReader: ';
      var mode = S.dict(arguments[0], w, 1).mode;
      if (mode !== undefined && (mode = String(mode)) !== 'byob') {
        throw new TypeError(w + "'" + mode + "' (value of 'mode' member of "
          + 'ReadableStreamGetReaderOptions) is not a valid value for enumeration '
          + 'ReadableStreamReaderMode.');
      }
      if (S.locked(this)) {
        throw new TypeError(w + 'Cannot get a new reader for a readable stream already locked by '
          + 'another reader.');
      }
      if (mode !== 'byob') return readerInit(Object.create(Reader.prototype), this, false);
      if (!slots(slots(this).controller).bytes) {
        throw new TypeError(w + 'Trying to read with incompatible controller');
      }
      return readerInit(Object.create(BYOB.prototype), this, true);
    },
  }, ['cancel']);
  Object.assign(S, { reader: function (stream) {
    return readerInit(Object.create(Reader.prototype), stream, false);
  }, read: read, release: release, request: request });
})(globalThis);
