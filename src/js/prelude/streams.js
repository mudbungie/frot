// Streams (bl-96f7, js.md §16): the WHATWG Streams standard, built as its
// abstract operations say — not a stub. Cloudflare's managed challenge reads
// `ReadableStream.prototype` (challenge.md §11.7). This file is the core the
// other four stream files (streambyte.js, streampipe.js, streamwrite.js,
// streamxform.js) share through the `__frot_streams` object, which the last of
// them removes: ReadableStream, its default controller and default reader.
// Member order, `.length`s and every error message were read off Firefox
// 153.0esr (identity.md §3.19 rig; js.md §16). Every internal slot lives in
// brand.js's one WeakMap, so an instance owns nothing.
(function (g) {
  'use strict';
  var slots = g.__frot_slots, iface = g.__frot_iface;

  function noop() {}
  function defer() {
    var d = {};
    d.promise = new Promise(function (res, rej) {
      d.resolve = res;
      d.reject = rej;
    });
    return d;
  }
  // A rejection nobody is obliged to await (the spec's "set handled to true").
  function handle(p) {
    p.catch(noop);
    return p;
  }
  // Call `f` on `self`; a throw becomes a rejection (WebIDL's promise-returning
  // callback invocation). No `f` is a promise resolved with undefined.
  function pcall(f, self, args) {
    try {
      return Promise.resolve(f ? f.apply(self, args) : undefined);
    } catch (e) {
      return Promise.reject(e);
    }
  }
  function isObj(v) {
    return v !== null && (typeof v === 'object' || typeof v === 'function');
  }
  // A dictionary argument: undefined/null are empty, a primitive is refused.
  function dict(v, at, n) {
    if (v === undefined || v === null) return {};
    if (!isObj(v)) throw new TypeError(at + 'Argument ' + n + " can't be converted to a dictionary.");
    return v;
  }
  // A callback member of a dictionary (`start`, `write`, `size`, …).
  function callback(d, key, at, name) {
    var f = d[key];
    if (f === undefined || typeof f === 'function') return f;
    throw new TypeError(at + "'" + key + "' member of " + name + ' is not '
      + (isObj(f) ? 'callable.' : 'an object.'));
  }
  // A QueuingStrategy argument, converted when the arguments are (size's type
  // first); the high-water mark is judged later, by `mark`.
  function strategy(v, at, n) {
    var d = dict(v, at, n);
    var hwm = d.highWaterMark;
    return { hwm: hwm === undefined ? undefined : Number(hwm),
      size: callback(d, 'size', at, 'QueuingStrategy') };
  }
  function mark(s, fallback, at) {
    if (s.hwm === undefined) return fallback;
    if (isNaN(s.hwm) || s.hwm < 0) throw new RangeError(at + 'Invalid highWaterMark');
    return s.hwm;
  }
  function sizer(s) {
    var size = s.size;
    return size ? function (chunk) {
      return size.call(undefined, chunk);
    } : function () {
      return 1;
    };
  }
  // A queue-with-sizes, kept in a controller's slots as `queue` and `total`.
  function enqueueSized(cs, v, size, at) {
    size = Number(size);
    if (!(size >= 0) || size === Infinity) throw new RangeError(at + 'invalid size');
    cs.queue.push({ v: v, size: size });
    cs.total += size;
  }
  function dequeue(cs) {
    var it = cs.queue.shift();
    cs.total = Math.max(cs.total - it.size, 0);
    return it.v;
  }
  function reset(cs) {
    cs.queue = [];
    cs.total = 0;
  }
  // --- ReadableStream: the stream's own state ------------------------------
  function init(stream) {
    Object.assign(slots(stream), { tag: 'RS', state: 'readable', reader: undefined,
      storedError: undefined, disturbed: false, controller: undefined });
    return stream;
  }
  function locked(s) {
    return slots(s).reader !== undefined;
  }
  function requests(s) {
    var r = slots(s).reader;
    return r ? slots(r).requests.length : 0;
  }
  function fulfill(stream, chunk, done) {
    var req = slots(slots(stream).reader).requests.shift();
    if (done) req.close(chunk);
    else req.chunk(chunk);
  }
  function streamClose(stream) {
    var ss = slots(stream);
    ss.state = 'closed';
    var r = ss.reader;
    if (!r) return;
    var rs = slots(r);
    rs.closed.resolve();
    if (rs.byob) return;
    rs.requests.splice(0).forEach(function (q) {
      q.close();
    });
  }
  function streamError(stream, e) {
    var ss = slots(stream);
    ss.state = 'errored';
    ss.storedError = e;
    var r = ss.reader;
    if (!r) return;
    var rs = slots(r);
    rs.closed.reject(e);
    handle(rs.closed.promise);
    rs.requests.splice(0).forEach(function (q) {
      q.error(e);
    });
  }
  function streamCancel(stream, reason) {
    var ss = slots(stream);
    ss.disturbed = true;
    if (ss.state === 'closed') return Promise.resolve();
    if (ss.state === 'errored') return Promise.reject(ss.storedError);
    streamClose(stream);
    var r = ss.reader;
    if (r && slots(r).byob) {
      slots(r).requests.splice(0).forEach(function (q) {
        q.close(undefined);
      });
    }
    var c = ss.controller;
    return slots(c).cancelSteps(c, reason).then(noop);
  }

  // --- The default controller ----------------------------------------------
  function canEnqueue(c) {
    var cs = slots(c);
    return !cs.closeRequested && slots(cs.stream).state === 'readable';
  }
  function desired(c) {
    var cs = slots(c), st = slots(cs.stream).state;
    return st === 'errored' ? null : st === 'closed' ? 0 : cs.hwm - cs.total;
  }
  function shouldPull(c) {
    var cs = slots(c);
    if (!canEnqueue(c) || !cs.started) return false;
    return (locked(cs.stream) && requests(cs.stream) > 0) || desired(c) > 0;
  }
  // Shared by the byte controller: its pull, queue and readers are shaped alike.
  function pullIfNeeded(c) {
    var cs = slots(c);
    if (!shouldPull(c)) return;
    if (cs.pulling) {
      cs.pullAgain = true;
      return;
    }
    cs.pulling = true;
    cs.pull(c).then(function () {
      cs.pulling = false;
      if (cs.pullAgain) {
        cs.pullAgain = false;
        pullIfNeeded(c);
      }
    }, function (e) {
      cs.error(c, e);
    });
  }
  function clearAlgos(c) {
    var cs = slots(c);
    cs.pull = cs.cancel = cs.size = undefined;
  }
  function dcClose(c) {
    if (!canEnqueue(c)) return;
    var cs = slots(c);
    cs.closeRequested = true;
    if (!cs.queue.length) {
      clearAlgos(c);
      streamClose(cs.stream);
    }
  }
  function dcEnqueue(c, chunk) {
    if (!canEnqueue(c)) return;
    var cs = slots(c);
    if (locked(cs.stream) && requests(cs.stream) > 0) fulfill(cs.stream, chunk, false);
    else {
      try {
        enqueueSized(cs, chunk, cs.size(chunk), 'ReadableStreamDefaultController.enqueue: ');
      } catch (e) {
        dcError(c, e);
        throw e;
      }
    }
    pullIfNeeded(c);
  }
  function dcError(c, e) {
    var cs = slots(c);
    if (slots(cs.stream).state !== 'readable') return;
    reset(cs);
    clearAlgos(c);
    streamError(cs.stream, e);
  }
  function dcPull(c, req) {
    var cs = slots(c);
    if (!cs.queue.length) {
      slots(slots(cs.stream).reader).requests.push(req);
      return pullIfNeeded(c);
    }
    var chunk = dequeue(cs);
    if (cs.closeRequested && !cs.queue.length) {
      clearAlgos(c);
      streamClose(cs.stream);
    } else pullIfNeeded(c);
    req.chunk(chunk);
  }
  function dcCancel(c, reason) {
    var cs = slots(c);
    reset(cs);
    var r = cs.cancel(reason);
    clearAlgos(c);
    return r;
  }
  var RSDC = iface('ReadableStreamDefaultController');
  // start/pull/cancel are the algorithms; `start` may throw out of the caller.
  function setupDefault(stream, start, pull, cancel, hwm, size) {
    var c = Object.create(RSDC.prototype);
    var cs = Object.assign(slots(c), { tag: 'RSDC', stream: stream, started: false,
      closeRequested: false, pulling: false, pullAgain: false, hwm: hwm, size: size,
      pull: pull, cancel: cancel, error: dcError, pullSteps: dcPull, cancelSteps: dcCancel,
      releaseSteps: noop });
    reset(cs);
    slots(stream).controller = c;
    Promise.resolve(start(c)).then(function () {
      cs.started = true;
      pullIfNeeded(c);
    }, function (e) {
      dcError(c, e);
    });
    return c;
  }
  // CreateReadableStream: a stream frot's own code drives (tee, from, a
  // TransformStream's readable side).
  function create(start, pull, cancel, hwm, size) {
    var s = init(Object.create(g.ReadableStream.prototype));
    setupDefault(s, start, pull, cancel, hwm === undefined ? 1 : hwm, size || function () {
      return 1;
    });
    return s;
  }

  g.__frot_streams = { noop: noop, defer: defer, handle: handle, pcall: pcall, isObj: isObj,
    dict: dict, callback: callback, strategy: strategy, mark: mark, sizer: sizer,
    enqueueSized: enqueueSized, dequeue: dequeue, reset: reset, init: init, locked: locked, requests: requests, fulfill: fulfill,
    streamClose: streamClose, streamError: streamError, streamCancel: streamCancel,
    canEnqueue: canEnqueue, desired: desired, shouldPull: shouldPull, pullIfNeeded: pullIfNeeded, clearAlgos: clearAlgos,
    dcClose: dcClose, dcEnqueue: dcEnqueue, dcError: dcError, setupDefault: setupDefault,
    create: create, RSDC: RSDC };
})(globalThis);
