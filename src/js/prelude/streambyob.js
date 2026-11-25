// Streams, part 4 (bl-96f7): the members of ReadableByteStreamController and
// ReadableStreamBYOBRequest, over streambyte.js's operations.
(function (g) {
  'use strict';
  var slots = g.__frot_slots, S = g.__frot_streams;
  var at = 'ReadableByteStreamController.', Controller = S.RBSC, Request = S.BYOBR;
  var close = S.byteClose, enqueue = S.byteEnqueue, error = S.byteError;
  var transfer = S.transfer, respondInternal = S.respond;

  function live(c) {
    var cs = slots(c);
    return !cs.closeRequested && slots(cs.stream).state === 'readable';
  }
  S.members(Controller.prototype, 'RBSC', 'ReadableByteStreamController', {
    close: function close_() {
      if (!live(this)) {
        throw new TypeError(at + 'close: Cannot close a stream that is already closed.');
      }
      close(this);
    },
    enqueue: function enqueue_(chunk) {
      var w = at + 'enqueue: ';
      S.need(arguments, 1, w);
      if (!S.isObj(chunk)) throw new TypeError(w + 'Argument 1 is not an object.');
      if (!ArrayBuffer.isView(chunk)) {
        throw new TypeError(w + 'Argument 1 does not implement interface ArrayBufferView.');
      }
      if (!chunk.byteLength) throw new TypeError(w + 'Zero Length View');
      if (!chunk.buffer.byteLength) throw new TypeError(w + 'Zero Length Buffer');
      if (!live(this)) throw new TypeError(w + 'Cannot enqueue into a stream that is already closed.');
      enqueue(this, chunk);
    },
    error: function error_() {
      error(this, arguments[0]);
    },
    get_byobRequest: function () {
      var cs = slots(this), first = cs.pending[0];
      if (!cs.byobRequest && first) {
        var r = Object.create(Request.prototype);
        Object.assign(slots(r), { tag: 'BYOBR', controller: this,
          view: new Uint8Array(first.buffer, first.byteOffset + first.bytesFilled,
            first.byteLength - first.bytesFilled) });
        cs.byobRequest = r;
      }
      return cs.byobRequest;
    },
    get_desiredSize: function () {
      return S.desired(this);
    },
  });
  var req = 'ReadableStreamBYOBRequest.';
  function owner(r, what) {
    var c = slots(r).controller;
    if (!c) throw new TypeError(req + what + ': This BYOB request has been invalidated');
    return c;
  }
  S.members(Request.prototype, 'BYOBR', 'ReadableStreamBYOBRequest', {
    respond: function respond(n) {
      S.need(arguments, 1, req + 'respond: ');
      var c = owner(this, 'respond'), first = slots(c).pending[0];
      n = Number(n);
      if (slots(slots(c).stream).state === 'closed') {
        if (n !== 0) throw new TypeError(req + 'respond: bytesWritten must be 0 for a closed stream');
      } else {
        if (n === 0) throw new TypeError(req + 'respond: bytesWritten must be greater than 0');
        if (first.bytesFilled + n > first.byteLength) {
          throw new RangeError(req + 'respond: bytesWritten out of range');
        }
      }
      first.buffer = transfer(first.buffer);
      respondInternal(c, n);
    },
    respondWithNewView: function respondWithNewView(v) {
      var w = req + 'respondWithNewView: ';
      S.need(arguments, 1, w);
      if (!ArrayBuffer.isView(v)) {
        throw new TypeError(w + 'Argument 1 could not be converted to any of: ArrayBufferView.');
      }
      var c = owner(this, 'respondWithNewView'), first = slots(c).pending[0];
      var closed = slots(slots(c).stream).state === 'closed';
      if (closed && v.byteLength) throw new TypeError(w + 'View must have zero length for a closed stream');
      if (!closed && !v.byteLength) throw new TypeError(w + 'View must have non-zero length');
      if (first.byteOffset + first.bytesFilled !== v.byteOffset
        || first.bufferByteLength !== v.buffer.byteLength
        || first.bytesFilled + v.byteLength > first.byteLength) {
        throw new RangeError(w + 'Invalid view');
      }
      first.buffer = transfer(v.buffer);
      respondInternal(c, v.byteLength);
    },
    get_view: function () {
      return slots(this).view;
    },
  });
})(globalThis);
