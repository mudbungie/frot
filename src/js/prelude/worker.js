// Worker / SharedWorker — coherent constructor PRESENCE without execution
// (js.md §7/§11, identity.md §10/§11, bl-342a). frot spawns no worker threads,
// but the §10 coherence bar makes absence a louder tell than a costume: real
// Firefox exposes these interfaces, so feature detection (`typeof Worker`,
// `'Worker' in window`, `new Worker(url)` shape) must see a Firefox-shaped
// surface. It does — and no thread is ever created, so a constructed
// Worker/SharedWorker simply never delivers a message. That non-delivery is the
// ONE declared residual (identity.md §11): honest silence, not a wrong value.
// Every value is fixed / profile-independent; no syscall (pure JS over brand.js,
// which this runs after — it needs __frot_brand / __frot_iface — and events.js,
// whose one EventTarget interface these three prototypes inherit).
(function (g) {
  'use strict';
  var onEvent = g.__frot_onevent;
  var brand = g.__frot_brand;

  // A branded native no-op. postMessage/terminate/port.close are real methods
  // that legally do nothing: there is no thread to receive a message or be torn
  // down. Returning undefined IS the honest outcome, not a lie (the residual).
  function noop(name) {
    return brand(function () {}, name);
  }


  // These interfaces ARE EventTargets, so they inherit the one interface in
  // events.js instead of owning flattened copies of its three methods. Measured
  // on Firefox 153.0esr (`bl-6438`, identity.md §3.14):
  //   Object.getPrototypeOf(Worker.prototype).constructor.name === 'EventTarget'
  // — the link frot lacked entirely, which also left `Worker.prototype` owning
  // eight properties where Firefox's owns six. Nothing frot does dispatches a
  // message to a worker (the residual is silence), but a page's OWN
  // `dispatchEvent` on one now behaves as a real EventTarget's does.
  function inheritEventTarget(Ctor) {
    Object.setPrototypeOf(Ctor.prototype, g.EventTarget.prototype);
  }

  // Gecko's WebIDL codegen defines `constructor` AFTER the interface's members,
  // so it comes LAST in `Object.getOwnPropertyNames(Worker.prototype)` (measured;
  // a JS function's `prototype` is born owning it, hence first). Re-defining it
  // moves it to the end. Applied ONLY to Worker — it is the one prototype whose
  // own-property list was read off the binary; the others stay as they are rather
  // than take an unmeasured shape (identity.md §3.14).
  function constructorLast(Ctor) {
    delete Ctor.prototype.constructor;
    Object.defineProperty(Ctor.prototype, 'constructor', {
      value: Ctor,
      configurable: true,
      writable: true,
    });
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

  // --- Worker: terminate/postMessage + the message-event handlers -------------
  // Definition order IS Firefox's own-property order, measured (`bl-6438`):
  //   ["terminate","postMessage","onmessage","onmessageerror","onerror","constructor"]
  var Worker = iface('Worker', function () {});
  inheritEventTarget(Worker);
  Worker.prototype.terminate = noop('terminate');
  Worker.prototype.postMessage = noop('postMessage');
  ['onmessage', 'onmessageerror', 'onerror'].forEach(function (k) {
    onEvent(Worker.prototype, k);
  });
  constructorLast(Worker);

  // --- MessagePort: NOT constructable (`new MessagePort()` throws "Illegal
  // constructor", so __frot_iface's throwing ctor is exactly right) — only a
  // SharedWorker's `port` hands one out. Same never-delivers residual.
  var MessagePort = g.__frot_iface('MessagePort', {});
  inheritEventTarget(MessagePort);
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
  inheritEventTarget(SharedWorker);
  Object.defineProperty(SharedWorker.prototype, 'port', {
    get: brand(function () { return this._port; }, 'get port'),
    enumerable: true,
    configurable: true,
  });
  onEvent(SharedWorker.prototype, 'onerror');
})(globalThis);
