// Streams, part 6 (bl-96f7): what a ReadableStream does with other streams —
// pipeThrough, pipeTo and tee. The writable side's operations are
// streamwrite.js's and streamwriter.js's (loaded before this one).
(function (g) {
  'use strict';
  var slots = g.__frot_slots, S = g.__frot_streams;

  function isRS(v) {
    return S.isObj(v) && slots(v).tag === 'RS';
  }
  function isWS(v) {
    return S.isObj(v) && slots(v).tag === 'WS';
  }
  // StreamPipeOptions, in WebIDL's member order.
  function options(v, at, n) {
    var d = S.dict(v, at, n);
    var o = { preventAbort: !!d.preventAbort, preventCancel: !!d.preventCancel,
      preventClose: !!d.preventClose, signal: d.signal };
    if (o.signal !== undefined && !(o.signal instanceof g.AbortSignal)) {
      throw new TypeError(at + "'signal' member of StreamPipeOptions does not implement "
        + 'interface AbortSignal.');
    }
    return o;
  }

  // ReadableStreamPipeTo: read while the writer is ready, write each chunk,
  // and on any end (either side closing or erroring, or the signal) shut down
  // once every chunk read has been written, propagating per the options.
  function pipe(source, dest, o) {
    var reader = S.reader(source), writer = S.writer(dest);
    var ss = slots(source), ds = slots(dest), d = S.defer();
    var down = false, current = Promise.resolve();
    ss.disturbed = true;
    function finalize(isErr, err) {
      S.writerRelease(writer);
      S.release(reader);
      if (o.signal) o.signal.removeEventListener('abort', onAbort);
      if (isErr) d.reject(err);
      else d.resolve();
    }
    function written() {
      var old = current;
      return current.then(function () {
        return old !== current ? written() : undefined;
      });
    }
    function shutdown(action, isErr, err) {
      if (down) return;
      down = true;
      var wait = ds.state === 'writable' && !S.closing(dest) ? written() : Promise.resolve();
      wait.then(function () {
        if (!action) return finalize(isErr, err);
        S.pcall(action).then(function () {
          finalize(isErr, err);
        }, function (e) {
          finalize(true, e);
        });
      });
    }
    function onAbort() {
      var e = o.signal.reason;
      shutdown(function () {
        // Firefox cancels the source before it aborts the destination (measured).
        return Promise.all([
          !o.preventCancel && ss.state === 'readable' ? S.streamCancel(source, e) : undefined,
          !o.preventAbort && ds.state === 'writable' ? S.wsAbort(dest, e) : undefined,
        ]);
      }, true, e);
    }
    function sourceErrored() {
      var e = ss.storedError;
      shutdown(o.preventAbort ? null : function () {
        return S.wsAbort(dest, e);
      }, true, e);
    }
    function destErrored() {
      var e = ds.storedError;
      shutdown(o.preventCancel ? null : function () {
        return S.streamCancel(source, e);
      }, true, e);
    }
    function destClosed() {
      var e = new TypeError('ReadableStream.pipeTo: Cannot pipe to a closing or closed stream.');
      shutdown(o.preventCancel ? null : function () {
        return S.streamCancel(source, e);
      }, true, e);
    }
    function step() {
      if (down) return;
      slots(writer).ready.promise.then(function () {
        if (down) return;
        S.read(reader, { chunk: function (c) {
          current = S.writerWrite(writer, c).then(S.noop, S.noop);
          step();
        }, close: S.noop, error: S.noop });
      }, S.noop);
    }
    if (o.signal) {
      if (o.signal.aborted) {
        onAbort();
        return d.promise;
      }
      o.signal.addEventListener('abort', onAbort);
    }
    if (ss.state === 'errored') sourceErrored();
    else if (ds.state === 'errored') destErrored();
    else if (ss.state === 'closed') shutdown(o.preventClose ? null : closeDest);
    else if (S.closing(dest) || ds.state === 'closed') destClosed();
    function closeDest() {
      return S.writerCloseProp(writer);
    }
    slots(reader).closed.promise.then(function () {
      shutdown(o.preventClose ? null : closeDest);
    }, sourceErrored);
    slots(writer).closed.promise.then(S.noop, destErrored);
    step();
    return d.promise;
  }

  // ReadableStreamDefaultTee (and, for a byte stream, its branches are byte
  // streams and the second gets a copy of each chunk).
  function tee(stream) {
    var reader = S.reader(stream), bytes = !!slots(slots(stream).controller).bytes;
    var reading = false, again = false, canceled = [false, false], reasons = [];
    var cancelled = S.defer(), branches = [];
    function each(f) {
      [0, 1].forEach(function (i) {
        if (!canceled[i]) f(slots(branches[i]).controller, i);
      });
    }
    function pull() {
      if (reading) {
        again = true;
        return Promise.resolve();
      }
      reading = true;
      S.read(reader, { chunk: function (chunk) {
        queueMicrotask(function () {
          again = false;
          var copy = bytes ? new Uint8Array(chunk) : chunk;
          each(function (c, i) {
            if (bytes) S.byteEnqueue(c, i ? copy : chunk);
            else S.dcEnqueue(c, chunk);
          });
          reading = false;
          if (again) pull();
        });
      }, close: function () {
        reading = false;
        each(function (c) {
          if (!bytes) return S.dcClose(c);
          S.byteClose(c);
          if (slots(c).pending.length) S.respond(c, 0);
        });
        if (!canceled[0] || !canceled[1]) cancelled.resolve();
      }, error: function () {
        reading = false;
      } });
      return Promise.resolve();
    }
    function cancel(i) {
      return function (reason) {
        canceled[i] = true;
        reasons[i] = reason;
        if (canceled[1 - i]) cancelled.resolve(S.streamCancel(stream, reasons));
        return cancelled.promise;
      };
    }
    [0, 1].forEach(function (i) {
      branches[i] = bytes ? S.createBytes(S.noop, pull, cancel(i))
        : S.create(S.noop, pull, cancel(i));
    });
    slots(reader).closed.promise.catch(function (e) {
      [0, 1].forEach(function (i) {
        var c = slots(branches[i]).controller;
        if (bytes) S.byteError(c, e);
        else S.dcError(c, e);
      });
      if (!canceled[0] || !canceled[1]) cancelled.resolve();
    });
    return branches;
  }

  var w = 'ReadableStream.';
  S.members(g.ReadableStream.prototype, 'RS', 'ReadableStream', {
    pipeThrough: function pipeThrough(transform) {
      var at = w + 'pipeThrough: ';
      S.need(arguments, 1, at);
      var t = S.dict(transform, at, 1), pair = ['readable', 'writable'];
      [t.readable, t.writable].forEach(function (v, i) {
        if (v === undefined) {
          throw new TypeError(at + "Missing required '" + pair[i] + "' member of ReadableWritablePair.");
        }
        if (!(i ? isWS : isRS)(v)) {
          throw new TypeError(at + "'" + pair[i] + "' member of ReadableWritablePair does not "
            + 'implement interface ' + (i ? 'WritableStream.' : 'ReadableStream.'));
        }
      });
      var o = options(arguments[1], at, 2);
      if (S.locked(this)) throw new TypeError(at + 'Cannot pipe from a locked stream.');
      if (S.wsLocked(t.writable)) throw new TypeError(at + 'Cannot pipe to a locked stream.');
      S.handle(pipe(this, t.writable, o));
      return t.readable;
    },
    pipeTo: function pipeTo(dest) {
      var at = w + 'pipeTo: ', o;
      try {
        S.need(arguments, 1, at);
        if (!isWS(dest)) throw new TypeError(at + 'Argument 1 does not implement interface WritableStream.');
        o = options(arguments[1], at, 2);
        if (S.locked(this)) throw new TypeError(at + 'Cannot pipe from a locked stream.');
        if (S.wsLocked(dest)) throw new TypeError(at + 'Cannot pipe to a locked stream.');
      } catch (e) {
        return Promise.reject(e);
      }
      return pipe(this, dest, o);
    },
    tee: function tee_() {
      return tee(this);
    },
  }, ['pipeTo']);
  Object.assign(S, { pipe: pipe, tee: tee, isRS: isRS, isWS: isWS });
})(globalThis);
