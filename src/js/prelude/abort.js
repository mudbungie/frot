// AbortController / AbortSignal (bl-e81b): Firefox 140esr's abort surface.
// framer-motion's lazy feature chunk constructs `new AbortController` bare
// (react-router carries its own typeof-guarded fallback), so absence is a
// ReferenceError that kills the route render into the app's error boundary.
// Signals are EventTarget instances riding loop.js's one listener registry;
// abort() flips state once, then fires onabort and the 'abort' listeners.
// Runs after loop.js (needs g.EventTarget, g.Event, and the virtual-clock
// setTimeout for AbortSignal.timeout). Deliberate residual: no subfetch
// integration — frot's network policy is once-then-frozen (js.md §6), nothing
// in flight is cancellable, so aborting only marks the signal, which is
// exactly the fact these libraries read back.
(function (g) {
  'use strict';

  // DOMException-shaped errors (quickjs lacks DOMException; the crypto.js tell).
  function named(name, message) {
    var e = new Error(message);
    e.name = name;
    return e;
  }

  function Signal() {
    g.EventTarget.call(this);
    this.aborted = false;
    this.reason = undefined;
    this.onabort = null;
  }
  Signal.prototype.throwIfAborted = function () {
    if (this.aborted) throw this.reason;
  };

  function doAbort(sig, reason) {
    if (sig.aborted) return;
    sig.aborted = true;
    sig.reason = reason;
    var ev = new g.Event('abort');
    ev.target = sig;
    if (typeof sig.onabort === 'function')
      try {
        sig.onabort.call(sig, ev);
      } catch (e) {}
    sig.dispatchEvent(ev);
  }

  g.AbortController = function AbortController() {
    this.signal = new Signal();
  };
  g.AbortController.prototype.abort = function (reason) {
    doAbort(this.signal, reason !== undefined ? reason : named('AbortError', 'The operation was aborted. '));
  };

  Signal.abort = function (reason) {
    var s = new Signal();
    s.aborted = true;
    s.reason = reason !== undefined ? reason : named('AbortError', 'The operation was aborted. ');
    return s;
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
    var s = new Signal();
    for (var i = 0; i < list.length; i++)
      if (list[i].aborted) {
        s.aborted = true;
        s.reason = list[i].reason;
        return s;
      }
    list.forEach(function (src) {
      src.addEventListener('abort', function () {
        doAbort(s, src.reason);
      });
    });
    return s;
  };
  g.AbortSignal = Signal;
})(globalThis);
