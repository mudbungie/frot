// AbortController / AbortSignal (bl-e81b): Firefox 140esr's abort surface.
// framer-motion's lazy feature chunk constructs `new AbortController` bare
// (react-router carries its own typeof-guarded fallback), so absence is a
// ReferenceError that kills the route render into the app's error boundary.
// Signals inherit EventTarget, riding events.js's one listener registry;
// abort() flips state once, then fires onabort and the 'abort' listeners.
// Runs after events.js (needs g.EventTarget / g.Event) and loop.js (the virtual-clock
// setTimeout for AbortSignal.timeout). Deliberate residual: no subfetch
// integration — frot's network policy is once-then-frozen (js.md §6), nothing
// in flight is cancellable, so aborting only marks the signal, which is
// exactly the fact these libraries read back.
(function (g) {
  'use strict';

  // DOMException-shaped errors (quickjs lacks DOMException) — brand.js's one
  // maker (bl-273b).
  var named = g.__frot_domerror;
  var attrs = g.__frot_ifaceattrs;
  // `aborted`/`reason`/`onabort` were instance DATA — three own properties on
  // every signal, where Gecko answers them from `AbortSignal.prototype`
  // accessors and the instance owns nothing. They are prototype accessors here
  // now, over brand.js's one instance-state WeakMap (bl-3bdc, §3.16).
  var slots = g.__frot_slots;

  // An AbortSignal IS an EventTarget — in Firefox by inheritance, and here too
  // since `bl-6438` moved the listener trio onto `EventTarget.prototype`: a
  // signal's `addEventListener` is now the ONE implementation it inherits, not
  // three methods stamped onto each instance by the constructor.
  function Signal() {
    var st = slots(this);
    st.aborted = false;
    st.reason = undefined;
  }
  Signal.prototype = Object.create(g.EventTarget.prototype);
  Object.defineProperty(Signal.prototype, 'constructor', {
    value: Signal,
    configurable: true,
    writable: true,
  });
  Signal.prototype.throwIfAborted = function () {
    if (this.aborted) throw this.reason;
  };
  attrs(Signal.prototype, ['aborted', 'reason']);
  g.__frot_onevent(Signal.prototype, 'onabort');

  // The one place a signal flips: state moves, then the handler, then the
  // listeners — the delivery order a real signal has.
  function doAbort(sig, reason) {
    var st = slots(sig);
    if (st.aborted) return;
    st.aborted = true;
    st.reason = reason;
    var ev = new g.Event('abort');
    if (typeof sig.onabort === 'function')
      try {
        sig.onabort.call(sig, ev);
      } catch (e) {}
    sig.dispatchEvent(ev);
  }
  // A pre-aborted signal: the same flip, minus the dispatch nobody can be
  // listening for yet — one path, not a second hand-set of the two slots.
  function aborted(reason) {
    var s = new Signal();
    slots(s).aborted = true;
    slots(s).reason = reason;
    return s;
  }
  function orAbortError(reason) {
    return reason !== undefined ? reason : named('AbortError', 'The operation was aborted. ');
  }

  var Controller = function AbortController() {
    slots(this).signal = new Signal();
  };
  // Operation before attribute: that is the order Gecko's WebIDL codegen emits
  // members in on all 41 prototypes read in `bl-706b` (identity.md §3.15).
  Controller.prototype.abort = function (reason) {
    doAbort(this.signal, orAbortError(reason));
  };
  attrs(Controller.prototype, ['signal']);
  g.AbortController = Controller;

  Signal.abort = function (reason) {
    return aborted(orAbortError(reason));
  };
  Signal.timeout = function (ms) {
    var s = new Signal();
    g.setTimeout(function () {
      doAbort(s, named('TimeoutError', 'The operation timed out.'));
    }, ms);
    return s;
  };
  Signal.any = function (signals) {
    var list = Array.prototype.slice.call(signals);
    for (var i = 0; i < list.length; i++)
      if (list[i].aborted) return aborted(list[i].reason);
    var s = new Signal();
    list.forEach(function (src) {
      src.addEventListener('abort', function () {
        doAbort(s, src.reason);
      });
    });
    return s;
  };
  g.AbortSignal = Signal;
})(globalThis);
