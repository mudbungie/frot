// Worker / SharedWorker — coherent constructor PRESENCE without execution
// (js.md §7/§11, identity.md §10/§11, bl-342a). frot spawns no worker threads,
// but the §10 coherence bar makes absence a louder tell than a costume: real
// Firefox exposes these interfaces, so feature detection (`typeof Worker`,
// `'Worker' in window`, `new Worker(url)` shape) must see a Firefox-shaped
// surface. It does — and no thread is ever created, so a constructed
// Worker/SharedWorker simply never delivers a message. That non-delivery is the
// ONE declared residual (identity.md §11): honest silence, not a wrong value.
// Every value is fixed / profile-independent; no syscall (pure JS over brand.js,
// which this runs after — it needs __frot_brand / __frot_iface).
(function (g) {
  'use strict';
  var brand = g.__frot_brand;

  // A branded native no-op. postMessage/terminate/port.close are real methods
  // that legally do nothing: there is no thread to receive a message or be torn
  // down. Returning undefined IS the honest outcome, not a lie (the residual).
  function noop(name) {
    return brand(function () {}, name);
  }

  // An event-handler IDL attribute (onmessage/onerror/…): an enumerable, native
  // accessor on the prototype defaulting to null, storing its handler in a
  // non-enumerable per-instance slot — exactly Firefox's shape. It is readable
  // and settable; it simply never fires, because no thread dispatches to it.
  function onEvent(proto, key) {
    var slot = '_on_' + key;
    Object.defineProperty(proto, key, {
      get: brand(function () {
        return Object.prototype.hasOwnProperty.call(this, slot) ? this[slot] : null;
      }, 'get ' + key),
      set: brand(function (fn) {
        Object.defineProperty(this, slot, {
          value: typeof fn === 'function' ? fn : null,
          configurable: true,
          writable: true,
        });
      }, 'set ' + key),
      enumerable: true,
      configurable: true,
    });
  }

  // The EventTarget methods Worker/SharedWorker/MessagePort inherit: present and
  // native. add/remove accept and discard — nothing frot does will ever dispatch
  // a message event to a worker, so retaining listeners would be dead state a
  // page could never observe fire. A page may still dispatchEvent its own
  // synthetic event; with no registered listener there is nothing to run, and it
  // returns true (the event was not canceled), matching EventTarget's contract.
  function eventTarget(proto) {
    proto.addEventListener = noop('addEventListener');
    proto.removeEventListener = noop('removeEventListener');
    proto.dispatchEvent = brand(function () { return true; }, 'dispatchEvent');
  }

  // Build a CONSTRUCTABLE Firefox-shaped interface `name` — unlike __frot_iface,
  // whose constructor throws "Illegal constructor". @@toStringTag on the
  // prototype gives `[object <name>]`; the constructor is published as a
  // NON-enumerable global (as browsers expose their interfaces); `build` shapes
  // each fresh instance. Firefox throws a TypeError when the required scriptURL
  // argument is missing, so this mirrors that one synchronous, observable check
  // and otherwise accepts any URL without fetching it (there is no thread to run
  // it) — the least-detectable, most honest masquerade. Returns the constructor.
  function iface(name, build) {
    var holder = {};
    holder[name] = function () {
      if (arguments.length < 1) {
        throw new TypeError(
          name + ' constructor: At least 1 argument required, but only 0 passed'
        );
      }
      build(this);
    };
    var Ctor = brand(holder[name], name);
    Object.defineProperty(Ctor.prototype, Symbol.toStringTag, {
      value: name,
      configurable: true,
    });
    Object.defineProperty(g, name, { value: Ctor, configurable: true, writable: true });
    return Ctor;
  }

  // --- Worker: postMessage/terminate + the message-event handlers --------------
  var Worker = iface('Worker', function () {});
  eventTarget(Worker.prototype);
  Worker.prototype.postMessage = noop('postMessage');
  Worker.prototype.terminate = noop('terminate');
  ['onmessage', 'onmessageerror', 'onerror'].forEach(function (k) {
    onEvent(Worker.prototype, k);
  });

  // --- MessagePort: NOT constructable (`new MessagePort()` throws "Illegal
  // constructor", so __frot_iface's throwing ctor is exactly right) — only a
  // SharedWorker's `port` hands one out. Same never-delivers residual.
  var MessagePort = g.__frot_iface('MessagePort', {});
  eventTarget(MessagePort.prototype);
  MessagePort.prototype.postMessage = noop('postMessage');
  MessagePort.prototype.start = noop('start');
  MessagePort.prototype.close = noop('close');
  ['onmessage', 'onmessageerror'].forEach(function (k) {
    onEvent(MessagePort.prototype, k);
  });

  // --- SharedWorker: a `port` (a fresh MessagePort) + onerror ------------------
  var SharedWorker = iface('SharedWorker', function (inst) {
    Object.defineProperty(inst, '_port', {
      value: Object.create(MessagePort.prototype),
      configurable: true,
    });
  });
  eventTarget(SharedWorker.prototype);
  Object.defineProperty(SharedWorker.prototype, 'port', {
    get: brand(function () { return this._port; }, 'get port'),
    enumerable: true,
    configurable: true,
  });
  onEvent(SharedWorker.prototype, 'onerror');
})(globalThis);
