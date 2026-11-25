// Streams, part 7 (bl-96f7): a ReadableStream as an async iterable — `values`,
// `[Symbol.asyncIterator]`, `for await` — and `ReadableStream.from`; then
// `locked`, last of the stream's members as measured.
(function (g) {
  'use strict';
  var slots = g.__frot_slots, S = g.__frot_streams;

  // --- Async iteration -------------------------------------------------------
  var AsyncIteratorProto = Object.getPrototypeOf(Object.getPrototypeOf(async function* () {}).prototype);
  var IterProto = Object.create(AsyncIteratorProto);
  // WebIDL's ongoing-promise chain. With nothing in flight the steps run at
  // once rather than a microtask later: QuickJS does not await `return()` when
  // a `for await` loop breaks, and Firefox, which does, has released the
  // reader by the loop's next line.
  function chain(it, steps) {
    var st = slots(it);
    var p = st.busy ? st.ongoing.then(steps, steps) : Promise.resolve(steps());
    var mine = st.ongoing = p;
    st.busy = true;
    function idle() {
      if (st.ongoing === mine) st.busy = false;
    }
    p.then(idle, idle);
    return p;
  }
  g.__frot_ifaceops(IterProto, {
    next: function next() {
      var it = this, st = slots(it);
      if (st.tag !== 'RSI') return Promise.reject(new TypeError('next called on an invalid iterator'));
      return chain(it, function () {
        if (st.finished) return { done: true, value: undefined };
        var d = S.defer();
        S.read(st.reader, { chunk: function (v) {
          d.resolve({ done: false, value: v });
        }, close: function () {
          st.finished = true;
          S.release(st.reader);
          d.resolve({ done: true, value: undefined });
        }, error: function (e) {
          st.finished = true;
          S.release(st.reader);
          d.reject(e);
        } });
        return d.promise;
      });
    },
    return: function return_(value) {
      var it = this, st = slots(it);
      if (st.tag !== 'RSI') return Promise.reject(new TypeError('return called on an invalid iterator'));
      return chain(it, function () {
        if (st.finished) return { done: true, value: value };
        st.finished = true;
        var stream = slots(st.reader).stream;
        var p = st.preventCancel ? Promise.resolve() : S.streamCancel(stream, value);
        S.release(st.reader);
        return p.then(function () {
          return { done: true, value: value };
        });
      });
    },
  });
  Object.defineProperty(IterProto, Symbol.toStringTag,
    { value: 'ReadableStream AsyncIterator', configurable: true });

  // --- ReadableStream.from ---------------------------------------------------
  function from(source) {
    var am = source == null ? undefined : source[Symbol.asyncIterator], it, sync = false;
    if (am == null) {
      am = source == null ? undefined : source[Symbol.iterator];
      sync = true;
    }
    if (typeof am !== 'function') throw new TypeError(String(am) + ' is not a function');
    it = am.call(source);
    if (!S.isObj(it)) throw new TypeError('ReadableStream.from: iterator is not an object');
    var nextFn = it.next, stream;
    return (stream = S.create(S.noop, function () {
      var c = slots(stream).controller;
      return S.pcall(nextFn, it, []).then(function (r) {
        if (!S.isObj(r)) throw new TypeError('ReadableStream.from: iterator result is not an object');
        return Promise.resolve(sync ? r.value : undefined).then(function (v) {
          if (r.done) S.dcClose(c);
          else S.dcEnqueue(c, sync ? v : r.value);
        });
      });
    }, function (reason) {
      var ret = it.return;
      if (ret == null) return Promise.resolve();
      return S.pcall(ret, it, [reason]).then(function (r) {
        if (!S.isObj(r)) throw new TypeError('ReadableStream.from: return result is not an object');
      });
    }, 0));
  }

  S.members(g.ReadableStream.prototype, 'RS', 'ReadableStream', {
    values: function values() {
      var prevent = !!S.dict(arguments[0], 'ReadableStream.values: ', 1).preventCancel;
      if (S.locked(this)) {
        throw new TypeError('Asynchronous iterator initialization steps for ReadableStream failed: '
          + 'Cannot get a new reader for a readable stream already locked by another reader.');
      }
      var it = Object.create(IterProto);
      Object.assign(slots(it), { tag: 'RSI', reader: S.reader(this), preventCancel: prevent,
        finished: false, busy: false, ongoing: null });
      return it;
    },
    get_locked: function () {
      return S.locked(this);
    },
  });
  var proto = g.ReadableStream.prototype;
  Object.defineProperty(proto, Symbol.asyncIterator,
    { value: proto.values, writable: true, configurable: true });
  g.__frot_ifaceops(g.ReadableStream, { from: from });
})(globalThis);
