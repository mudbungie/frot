// Streams, part 5 (bl-96f7): WritableStreamDefaultWriter, and the members of
// WritableStream and its controller, over streamwrite.js's state machine.
(function (g) {
  'use strict';
  var slots = g.__frot_slots, S = g.__frot_streams;
  var locked = S.wsLocked, closing = S.closing, abort = S.wsAbort, close = S.wsClose;
  var desired = S.wsDesired, settled = S.settled, rejectReady = S.rejectReady;
  var rejectClosed = S.rejectClosed, advance = S.advance, errorIfNeeded = S.wsErrorIfNeeded;
  var Controller = S.WSDC, WritableStream = g.WritableStream;

  // --- The writer ------------------------------------------------------------
  function writer(s, w) {
    w = w || Object.create(Writer.prototype);
    var ss = slots(s);
    var ws = Object.assign(slots(w), { tag: 'WSDW', stream: s });
    ss.writer = w;
    var st = ss.state, e = ss.storedError;
    ws.readySettled = !(st === 'writable' && !closing(s) && ss.backpressure);
    ws.ready = ws.readySettled ? settled(st === 'erroring' || st === 'errored', e) : S.defer();
    ws.closedSettled = st === 'closed' || st === 'errored';
    ws.closed = ws.closedSettled ? settled(st === 'errored', e) : S.defer();
    return w;
  }
  function release(w) {
    var ws = slots(w), e = new TypeError('Releasing lock');
    rejectReady(w, e);
    rejectClosed(w, e);
    slots(ws.stream).writer = undefined;
    ws.stream = undefined;
  }
  function write(w, chunk) {
    var s = slots(w).stream, ss = slots(s), c = ss.controller, cs = slots(c), size = 1;
    try {
      if (cs.size) size = cs.size(chunk);
    } catch (e) {
      errorIfNeeded(c, e);
    }
    if (s !== slots(w).stream) return Promise.reject(new TypeError('Writer was released'));
    if (ss.state === 'errored') return Promise.reject(ss.storedError);
    if (closing(s) || ss.state === 'closed') {
      return Promise.reject(new TypeError('Stream is closed or closing'));
    }
    if (ss.state === 'erroring') return Promise.reject(ss.storedError);
    var d = S.defer();
    ss.writes.push(d);
    try {
      S.enqueueSized(cs, chunk, size, 'WritableStreamDefaultWriter.write: ');
    } catch (e) {
      errorIfNeeded(c, e);
      return d.promise;
    }
    if (!closing(s) && ss.state === 'writable') S.updateBackpressure(s, desired(c) <= 0);
    advance(c);
    return d.promise;
  }
  function closeProp(w) {
    var s = slots(w).stream, st = slots(s).state;
    if (closing(s) || st === 'closed') return Promise.resolve();
    if (st === 'errored') return Promise.reject(slots(s).storedError);
    return close(s);
  }
  var Writer = S.ctor('WritableStreamDefaultWriter', 1, function (inst, args) {
    var w = 'WritableStreamDefaultWriter constructor: ';
    S.need(args, 1, w);
    if (!S.isObj(args[0]) || slots(args[0]).tag !== 'WS') {
      throw new TypeError(w + 'Argument 1 does not implement interface WritableStream.');
    }
    if (locked(args[0])) throw new TypeError(w + 'WritableStream is already locked!');
    writer(args[0], inst);
  });
  var gone = 'WritableStreamDefaultWriter.';
  function released(what) {
    return Promise.reject(new TypeError(gone + what + ': Missing stream'));
  }
  S.members(Writer.prototype, 'WSDW', 'WritableStreamDefaultWriter', {
    abort: function abort_() {
      return slots(this).stream ? abort(slots(this).stream, arguments[0]) : released('abort');
    },
    close: function close_() {
      var s = slots(this).stream;
      if (!s) return released('close');
      if (closing(s)) return Promise.reject(new TypeError('Stream is closed or closing'));
      return close(s);
    },
    releaseLock: function releaseLock() {
      if (slots(this).stream) release(this);
    },
    write: function write_() {
      return slots(this).stream ? write(this, arguments[0]) : released('write');
    },
    get_closed: function () {
      return slots(this).closed.promise;
    },
    get_desiredSize: function () {
      var s = slots(this).stream;
      if (!s) throw new TypeError(gone + 'desiredSize getter: Missing stream');
      var st = slots(s).state;
      if (st === 'errored' || st === 'erroring') return null;
      return st === 'closed' ? 0 : desired(slots(s).controller);
    },
    get_ready: function () {
      return slots(this).ready.promise;
    },
  }, ['abort', 'close', 'write']);

  S.members(Controller.prototype, 'WSDC', 'WritableStreamDefaultController', {
    error: function error() {
      if (slots(slots(this).stream).state === 'writable') errorIfNeeded(this, arguments[0]);
    },
    get_signal: function () {
      return slots(this).signal.signal;
    },
  });
  var ws = 'WritableStream.';
  S.members(WritableStream.prototype, 'WS', 'WritableStream', {
    abort: function abort_() {
      if (locked(this)) {
        return Promise.reject(new TypeError('Canceled Locked Stream'));
      }
      return abort(this, arguments[0]);
    },
    close: function close_() {
      if (locked(this)) {
        return Promise.reject(new TypeError('Can not close locked stream'));
      }
      if (closing(this)) return Promise.reject(new TypeError('Stream is closed or closing'));
      return close(this);
    },
    getWriter: function getWriter() {
      if (locked(this)) throw new TypeError(ws + 'getWriter: WritableStream is already locked!');
      return writer(this);
    },
    get_locked: function () {
      return locked(this);
    },
  }, ['abort', 'close']);
  Object.assign(S, { writer: writer, writerRelease: release, writerWrite: write,
    writerCloseProp: closeProp });
})(globalThis);
