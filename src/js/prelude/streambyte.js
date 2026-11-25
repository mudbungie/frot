// Streams, part 3 (bl-96f7): the byte stream — ReadableByteStreamController
// and ReadableStreamBYOBRequest. A `Response.body` and a `Blob.stream()` are
// byte streams in Firefox (a BYOB reader works on both — measured), so this is
// what those are made of. A pull-into descriptor is the spec's: a reader's
// buffer being filled, `bytesFilled` of `byteLength`, ready at `minimumFill`.
// Enqueued and read buffers are transferred (the caller's view detaches), as
// measured.
(function (g) {
  'use strict';
  var slots = g.__frot_slots, S = g.__frot_streams;
  var at = 'ReadableByteStreamController.';

  function transfer(buf) {
    return typeof buf.transfer === 'function' ? buf.transfer() : buf.slice(0);
  }
  function hasReader(stream, byob) {
    var r = slots(stream).reader;
    return !!r && slots(r).byob === byob;
  }
  function invalidate(c) {
    var cs = slots(c);
    if (!cs.byobRequest) return;
    Object.assign(slots(cs.byobRequest), { controller: undefined, view: null });
    cs.byobRequest = null;
  }
  function pushChunk(cs, buffer, off, len) {
    cs.queue.push({ buffer: buffer, off: off, len: len });
    cs.total += len;
  }
  function pushClone(cs, buffer, off, len) {
    pushChunk(cs, buffer.slice(off, off + len), 0, len);
  }
  function view(pi) {
    return new pi.ctor(pi.buffer, pi.byteOffset, pi.bytesFilled / pi.elementSize);
  }
  function commit(stream, pi) {
    var done = slots(stream).state === 'closed';
    if (pi.readerType !== 'none') S.fulfill(stream, view(pi), done);
  }
  // Copy what the queue holds into `pi`; true once it reaches its minimum.
  function fillFromQueue(c, pi) {
    var cs = slots(c);
    var most = Math.min(cs.total, pi.byteLength - pi.bytesFilled);
    var filled = pi.bytesFilled + most;
    var aligned = filled - (filled % pi.elementSize);
    var left = most, ready = aligned >= pi.minimumFill;
    if (ready) left = aligned - pi.bytesFilled;
    while (left > 0) {
      var head = cs.queue[0], n = Math.min(left, head.len);
      new Uint8Array(pi.buffer, pi.byteOffset + pi.bytesFilled, n)
        .set(new Uint8Array(head.buffer, head.off, n));
      if (head.len === n) cs.queue.shift();
      else {
        head.off += n;
        head.len -= n;
      }
      cs.total -= n;
      pi.bytesFilled += n;
      left -= n;
    }
    return ready;
  }
  function drain(c) {
    var cs = slots(c);
    if (cs.total === 0 && cs.closeRequested) {
      S.clearAlgos(c);
      S.streamClose(cs.stream);
    } else S.pullIfNeeded(c);
  }
  function fromQueue(c, req) {
    var cs = slots(c), e = cs.queue.shift();
    cs.total -= e.len;
    drain(c);
    req.chunk(new Uint8Array(e.buffer, e.off, e.len));
  }
  function usingQueue(c) {
    var cs = slots(c), out = [];
    while (cs.pending.length && cs.total) {
      if (fillFromQueue(c, cs.pending[0])) out.push(cs.pending.shift());
    }
    return out;
  }
  function commitAll(c, list) {
    list.forEach(function (pi) {
      commit(slots(c).stream, pi);
    });
  }
  function detached(c, pi) {
    var cs = slots(c);
    if (pi.bytesFilled > 0) pushClone(cs, pi.buffer, pi.byteOffset, pi.bytesFilled);
    cs.pending.shift();
  }

  function enqueue(c, chunk) {
    var cs = slots(c), stream = cs.stream;
    if (cs.closeRequested || slots(stream).state !== 'readable') return;
    var off = chunk.byteOffset, len = chunk.byteLength, buf = transfer(chunk.buffer);
    if (cs.pending.length) {
      invalidate(c);
      var first = cs.pending[0];
      first.buffer = transfer(first.buffer);
      if (first.readerType === 'none') detached(c, first);
    }
    if (hasReader(stream, false)) {
      var r = slots(slots(stream).reader).requests;
      while (r.length && cs.total) fromQueue(c, r.shift());
      if (!r.length) pushChunk(cs, buf, off, len);
      else {
        if (cs.pending.length) cs.pending.shift();
        S.fulfill(stream, new Uint8Array(buf, off, len), false);
      }
    } else if (hasReader(stream, true)) {
      pushChunk(cs, buf, off, len);
      commitAll(c, usingQueue(c));
    } else pushChunk(cs, buf, off, len);
    S.pullIfNeeded(c);
  }
  function close(c) {
    var cs = slots(c);
    if (cs.closeRequested || slots(cs.stream).state !== 'readable') return;
    if (cs.total > 0) {
      cs.closeRequested = true;
      return;
    }
    var first = cs.pending[0];
    if (first && first.bytesFilled % first.elementSize) {
      var e = new TypeError(at + 'close: Insufficient bytes to fill elements in the given buffer');
      error(c, e);
      throw e;
    }
    S.clearAlgos(c);
    S.streamClose(cs.stream);
  }
  function error(c, e) {
    var cs = slots(c);
    if (slots(cs.stream).state !== 'readable') return;
    invalidate(c);
    cs.pending = [];
    S.reset(cs);
    S.clearAlgos(c);
    S.streamError(cs.stream, e);
  }
  function respondInternal(c, n) {
    var cs = slots(c), first = cs.pending[0], stream = cs.stream;
    invalidate(c);
    if (slots(stream).state === 'closed') {
      if (first.readerType === 'none') cs.pending.shift();
      if (hasReader(stream, true)) {
        while (S.requests(stream) > 0) commit(stream, cs.pending.shift());
      }
    } else {
      first.bytesFilled += n;
      if (first.readerType === 'none') {
        detached(c, first);
        commitAll(c, usingQueue(c));
      } else if (first.bytesFilled >= first.minimumFill) {
        cs.pending.shift();
        var rem = first.bytesFilled % first.elementSize;
        if (rem) pushClone(cs, first.buffer, first.byteOffset + first.bytesFilled - rem, rem);
        first.bytesFilled -= rem;
        var rest = usingQueue(c);
        commit(stream, first);
        commitAll(c, rest);
      }
    }
    S.pullIfNeeded(c);
  }
  function pullInto(c, v, min, req) {
    var cs = slots(c), stream = cs.stream;
    var size = v.BYTES_PER_ELEMENT || 1;
    var pi = { byteOffset: v.byteOffset, byteLength: v.byteLength, bytesFilled: 0,
      minimumFill: min * size, elementSize: size, ctor: v.constructor, readerType: 'byob' };
    pi.buffer = transfer(v.buffer);
    pi.bufferByteLength = pi.buffer.byteLength;
    if (!cs.pending.length && slots(stream).state === 'closed') {
      return req.close(new pi.ctor(pi.buffer, pi.byteOffset, 0));
    }
    if (!cs.pending.length && cs.total > 0) {
      if (fillFromQueue(c, pi)) {
        drain(c);
        return req.chunk(view(pi));
      }
      if (cs.closeRequested) {
        var e = new TypeError('ReadableStreamBYOBReader.read: Insufficient bytes to fill elements '
          + 'in the given buffer');
        error(c, e);
        return req.error(e);
      }
    }
    cs.pending.push(pi);
    slots(slots(stream).reader).requests.push(req);
    S.pullIfNeeded(c);
  }
  function pullSteps(c, req) {
    var cs = slots(c);
    if (cs.total > 0) return fromQueue(c, req);
    if (cs.auto !== undefined) {
      cs.pending.push({ buffer: new ArrayBuffer(cs.auto), bufferByteLength: cs.auto, byteOffset: 0,
        byteLength: cs.auto, bytesFilled: 0, minimumFill: 1, elementSize: 1, ctor: Uint8Array,
        readerType: 'default' });
    }
    slots(slots(cs.stream).reader).requests.push(req);
    S.pullIfNeeded(c);
  }

  var Controller = g.__frot_iface('ReadableByteStreamController');
  var Request = g.__frot_iface('ReadableStreamBYOBRequest');
  function setupBytes(stream, start, pull, cancel, hwm, auto) {
    var c = Object.create(Controller.prototype);
    var cs = Object.assign(slots(c), { tag: 'RBSC', bytes: true, stream: stream, started: false,
      closeRequested: false, pulling: false, pullAgain: false, hwm: hwm, auto: auto, pull: pull,
      cancel: cancel, pending: [], byobRequest: null, error: error, pullSteps: pullSteps,
      cancelSteps: function (ctl, reason) {
        invalidate(ctl);
        cs.pending = [];
        S.reset(cs);
        var r = cs.cancel(reason);
        S.clearAlgos(ctl);
        return r;
      }, releaseSteps: function () {
        if (cs.pending.length) {
          cs.pending[0].readerType = 'none';
          cs.pending.length = 1;
        }
      } });
    S.reset(cs);
    slots(stream).controller = c;
    Promise.resolve(start(c)).then(function () {
      cs.started = true;
      S.pullIfNeeded(c);
    }, function (e) {
      error(c, e);
    });
    return c;
  }
  // A byte stream frot's own code drives (Response.body, Blob.stream, tee).
  function createBytes(start, pull, cancel) {
    var s = S.init(Object.create(g.ReadableStream.prototype));
    setupBytes(s, start, pull || function () {
      return Promise.resolve();
    }, cancel || function () {
      return Promise.resolve();
    }, 0, undefined);
    return s;
  }

  Object.assign(S, { setupBytes: setupBytes, createBytes: createBytes, pullInto: pullInto,
    byteEnqueue: enqueue, byteClose: close, byteError: error, respond: respondInternal,
    transfer: transfer, RBSC: Controller, BYOBR: Request });
})(globalThis);
