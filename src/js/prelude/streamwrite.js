// Streams, part 4 (bl-96f7): WritableStream, its default writer and default
// controller — the standard's state machine (writable → erroring → errored,
// or → closed), one write or close in flight at a time, backpressure through
// the writer's `ready`.
(function (g) {
  'use strict';
  var slots = g.__frot_slots, S = g.__frot_streams;
  var CLOSE = {}; // the close sentinel in the controller's queue
  var at = 'WritableStream constructor: ';

  function locked(s) {
    return slots(s).writer !== undefined;
  }
  function closing(s) {
    var ss = slots(s);
    return ss.closeRequest !== undefined || ss.inFlightClose !== undefined;
  }
  // A deferred settled now: resolved, or rejected (and handled) when `bad`.
  function settled(bad, e) {
    var d = S.defer();
    if (!bad) d.resolve();
    else {
      d.reject(e);
      S.handle(d.promise);
    }
    return d;
  }
  function desired(c) {
    var cs = slots(c);
    return cs.hwm - cs.total;
  }
  function updateBackpressure(s, bp) {
    var ss = slots(s), w = ss.writer;
    if (w && bp !== ss.backpressure) {
      if (bp) slots(w).ready = S.defer();
      else slots(w).ready.resolve();
      slots(w).readySettled = !bp;
    }
    ss.backpressure = bp;
  }
  function rejectReady(w, e) {
    var ws = slots(w);
    if (ws.readySettled) ws.ready = S.defer();
    ws.ready.reject(e);
    ws.readySettled = true;
    S.handle(ws.ready.promise);
  }
  function rejectClosed(w, e) {
    var ws = slots(w);
    if (ws.closedSettled) ws.closed = S.defer();
    ws.closed.reject(e);
    ws.closedSettled = true;
    S.handle(ws.closed.promise);
  }
  function abort(s, reason) {
    var ss = slots(s);
    if (ss.state === 'closed' || ss.state === 'errored') return Promise.resolve();
    slots(ss.controller).signal.abort(reason);
    if (ss.state === 'closed' || ss.state === 'errored') return Promise.resolve();
    if (ss.pendingAbort) return ss.pendingAbort.d.promise;
    var already = ss.state === 'erroring', d = S.defer();
    ss.pendingAbort = { d: d, reason: already ? undefined : reason, already: already };
    if (!already) startErroring(s, reason);
    return d.promise;
  }
  function close(s) {
    var ss = slots(s);
    if (ss.state === 'closed' || ss.state === 'errored') {
      return Promise.reject(new TypeError('Stream is closed or closing'));
    }
    var d = S.defer();
    ss.closeRequest = d;
    if (ss.writer && ss.backpressure && ss.state === 'writable') {
      slots(ss.writer).ready.resolve();
      slots(ss.writer).readySettled = true;
    }
    S.enqueueSized(slots(ss.controller), CLOSE, 0, '');
    advance(ss.controller);
    return d.promise;
  }
  function dealWithRejection(s, e) {
    if (slots(s).state === 'writable') startErroring(s, e);
    else finishErroring(s);
  }
  function startErroring(s, reason) {
    var ss = slots(s), c = ss.controller;
    ss.state = 'erroring';
    ss.storedError = reason;
    if (ss.writer) rejectReady(ss.writer, reason);
    if (!ss.inFlightWrite && !ss.inFlightClose && slots(c).started) finishErroring(s);
  }
  function rejectCloseAndClosed(s) {
    var ss = slots(s);
    if (ss.closeRequest) {
      ss.closeRequest.reject(ss.storedError);
      ss.closeRequest = undefined;
    }
    if (ss.writer) rejectClosed(ss.writer, ss.storedError);
  }
  function finishErroring(s) {
    var ss = slots(s), c = ss.controller;
    ss.state = 'errored';
    S.reset(slots(c));
    ss.writes.splice(0).forEach(function (d) {
      d.reject(ss.storedError);
    });
    var req = ss.pendingAbort;
    ss.pendingAbort = undefined;
    if (!req) return rejectCloseAndClosed(s);
    if (req.already) {
      req.d.reject(ss.storedError);
      return rejectCloseAndClosed(s);
    }
    var p = slots(c).abort(req.reason);
    clearAlgos(c);
    p.then(function () {
      req.d.resolve();
      rejectCloseAndClosed(s);
    }, function (e) {
      req.d.reject(e);
      rejectCloseAndClosed(s);
    });
  }
  function finishClose(s, e) {
    var ss = slots(s);
    if (e !== undefined) {
      ss.inFlightClose.reject(e);
      ss.inFlightClose = undefined;
      if (ss.pendingAbort) {
        ss.pendingAbort.d.reject(e);
        ss.pendingAbort = undefined;
      }
      return dealWithRejection(s, e);
    }
    ss.inFlightClose.resolve();
    ss.inFlightClose = undefined;
    if (ss.state === 'erroring') {
      ss.storedError = undefined;
      if (ss.pendingAbort) {
        ss.pendingAbort.d.resolve();
        ss.pendingAbort = undefined;
      }
    }
    ss.state = 'closed';
    if (ss.writer) {
      slots(ss.writer).closed.resolve();
      slots(ss.writer).closedSettled = true;
    }
  }

  // --- The controller --------------------------------------------------------
  function clearAlgos(c) {
    var cs = slots(c);
    cs.write = cs.close = cs.abort = cs.size = undefined;
  }
  function errorIfNeeded(c, e) {
    var s = slots(c).stream;
    if (slots(s).state === 'writable') {
      clearAlgos(c);
      startErroring(s, e);
    }
  }
  function advance(c) {
    var cs = slots(c), s = cs.stream, ss = slots(s);
    if (!cs.started || ss.inFlightWrite) return;
    if (ss.state === 'erroring') return finishErroring(s);
    if (!cs.queue.length) return;
    var v = cs.queue[0].v;
    if (v === CLOSE) {
      ss.inFlightClose = ss.closeRequest;
      ss.closeRequest = undefined;
      S.dequeue(cs);
      var p = cs.close();
      clearAlgos(c);
      return p.then(function () {
        finishClose(s);
      }, function (e) {
        finishClose(s, e);
      });
    }
    ss.inFlightWrite = ss.writes.shift();
    cs.write(v).then(function () {
      ss.inFlightWrite.resolve();
      ss.inFlightWrite = undefined;
      S.dequeue(cs);
      if (!closing(s) && ss.state === 'writable') updateBackpressure(s, desired(c) <= 0);
      advance(c);
    }, function (e) {
      if (ss.state === 'writable') clearAlgos(c);
      ss.inFlightWrite.reject(e);
      ss.inFlightWrite = undefined;
      dealWithRejection(s, e);
    });
  }
  var Controller = g.__frot_iface('WritableStreamDefaultController');
  function setup(s, start, write, closeAlgo, abortAlgo, hwm, size) {
    var c = Object.create(Controller.prototype);
    var cs = Object.assign(slots(c), { tag: 'WSDC', stream: s, started: false, hwm: hwm,
      size: size, write: write, close: closeAlgo, abort: abortAlgo,
      signal: new g.AbortController() });
    S.reset(cs);
    slots(s).controller = c;
    updateBackpressure(s, desired(c) <= 0);
    Promise.resolve(start(c)).then(function () {
      cs.started = true;
      advance(c);
    }, function (e) {
      cs.started = true;
      dealWithRejection(s, e);
    });
    return c;
  }
  function init(s) {
    return Object.assign(slots(s), { tag: 'WS', state: 'writable', storedError: undefined,
      writer: undefined, controller: undefined, writes: [], inFlightWrite: undefined,
      closeRequest: undefined, inFlightClose: undefined, pendingAbort: undefined,
      backpressure: false });
  }
  var WritableStream = S.ctor('WritableStream', 0, function (inst, args) {
    var sink = args[0];
    if (sink !== undefined && !S.isObj(sink)) {
      throw new TypeError(at + 'Argument 1 is not an object.');
    }
    var strat = S.strategy(args[1], at, 2);
    var d = sink === undefined ? {} : sink, dat = 'WritableStream.constructor: ';
    var ab = S.callback(d, 'abort', dat, 'UnderlyingSink');
    var cl = S.callback(d, 'close', dat, 'UnderlyingSink');
    var st = S.callback(d, 'start', dat, 'UnderlyingSink');
    var type = d.type;
    var wr = S.callback(d, 'write', dat, 'UnderlyingSink');
    if (type !== undefined) throw new RangeError(at + "Implementation preserved member 'type'");
    init(inst);
    setup(inst, function (c) {
      return st ? st.call(sink, c) : undefined;
    }, function (chunk) {
      return S.pcall(wr, sink, [chunk, slots(inst).controller]);
    }, function () {
      return S.pcall(cl, sink, []);
    }, function (reason) {
      return S.pcall(ab, sink, [reason]);
    }, S.mark(strat, 1, at), S.sizer(strat));
  });
  function create(start, write, closeAlgo, abortAlgo, hwm, size) {
    var s = Object.create(WritableStream.prototype);
    init(s);
    setup(s, start, write, closeAlgo, abortAlgo, hwm, size);
    return s;
  }

  Object.assign(S, { wsLocked: locked, closing: closing, wsAbort: abort, wsClose: close,
    wsCreate: create, wsErrorIfNeeded: errorIfNeeded, wsDesired: desired, settled: settled,
    rejectReady: rejectReady, rejectClosed: rejectClosed, advance: advance, WSDC: Controller,
    updateBackpressure: updateBackpressure });
})(globalThis);
