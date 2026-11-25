// The Event interfaces (bl-643d, bl-810b): Event, CustomEvent, ErrorEvent, and
// the one per-event state every dispatch reads and writes. The state lives in
// brand.js's slots — never in a property a page can walk — and its fields are
// the DOM standard's own (§2.2 "Interface Event"): the type, the bubbles /
// cancelable / composed flags, target / currentTarget / eventPhase, the path,
// and the stop-propagation, stop-immediate-propagation, canceled, initialized,
// dispatch and in-passive-listener flags. events.js's dispatch is the only
// writer of the dispatch-time fields. Measured on Firefox 153.0esr
// (identity.md §3.19 rig; src/js/tests/dispatch/firefox153.json).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;

  function argc(where, need, got) {
    if (got < need)
      throw new TypeError(where + ': At least ' + need + ' argument' + (need > 1 ? 's' : '')
        + ' required, but only ' + got + ' passed');
  }

  // A constructed `new Event('x')` owns `isTrusted` alone — an enumerable,
  // NON-configurable accessor reading `false` (identity.md §3.17) — and
  // everything else comes off `Event.prototype`. `init` is the EventInit
  // dictionary; `created` false is `document.createEvent`'s uninitialized event,
  // which dispatch refuses until `initEvent` runs.
  function initEvent(inst, type, init, created) {
    var st = slots(inst);
    st.event = true;
    st.initialized = created !== false;
    st.type = String(type);
    st.bubbles = !!(init && init.bubbles);
    st.cancelable = !!(init && init.cancelable);
    st.composed = !!(init && init.composed);
    st.defaultPrevented = false;
    st.target = null;
    st.currentTarget = null;
    st.eventPhase = 0;
    st.path = [];
    st.timeStamp = g.__frot_event_time ? g.__frot_event_time() : 0;
    Object.defineProperty(inst, 'isTrusted', {
      // False for anything a page constructs; true only where the host raises
      // the event itself and marks it (imageload.js, filereader.js, net.js).
      get: brand(function () {
        return st.trusted === true;
      }, 'get isTrusted'),
      enumerable: true,
      configurable: false,
    });
  }
  var Event = g.__frot_iface('Event', null, function (inst, args) {
    argc('Event constructor', 1, args.length);
    initEvent(inst, args[0], args[1]);
  });
  // The event subtypes in other modules (ProgressEvent, IDBVersionChangeEvent)
  // start from the same initializer; calling `Event` itself needs `new`.
  Object.defineProperty(g, '__frot_event_init', {
    value: brand(initEvent, '__frot_event_init'),
    configurable: true,
    writable: true,
  });

  // "Set the canceled flag" (DOM §2.2): only a cancelable event, and never
  // from inside a passive listener.
  function cancel(st) {
    if (st.cancelable && !st.passive) st.defaultPrevented = true;
  }
  g.__frot_ifaceops(Event.prototype, {
    // The path, from the target out, while the event is being dispatched;
    // empty before and after (no shadow trees: nothing is hidden from it).
    composedPath: function composedPath() {
      return slots(this).path.slice();
    },
    stopPropagation: function stopPropagation() {
      slots(this).stop = true;
    },
    stopImmediatePropagation: function stopImmediatePropagation() {
      var st = slots(this);
      st.stop = st.stopNow = true;
    },
    preventDefault: function preventDefault() {
      cancel(slots(this));
    },
    // A no-op while the event is dispatched (measured: the type survives).
    initEvent: function initEvent(type, bubbles, cancelable) {
      argc('Event.initEvent', 1, arguments.length);
      var st = slots(this);
      if (st.dispatching) return;
      st.initialized = true;
      st.stop = st.stopNow = st.defaultPrevented = false;
      st.type = String(type);
      st.bubbles = !!bubbles;
      st.cancelable = !!cancelable;
      st.target = null;
    },
  });
  function target() {
    return slots(this).target;
  }
  g.__frot_ifaceattrs(Event.prototype, {
    type: null,
    target: target,
    srcElement: target,
    currentTarget: null,
    eventPhase: null,
    bubbles: null,
    cancelable: null,
  });
  // The two legacy writable attributes: each write only ever SETS its flag —
  // `returnValue = true` does not un-cancel, `cancelBubble = false` does not
  // un-stop (measured).
  Object.defineProperty(Event.prototype, 'returnValue', {
    get: brand(function () {
      return !slots(this).defaultPrevented;
    }, 'get returnValue'),
    set: brand(function (v) {
      if (!v) cancel(slots(this));
    }, 'set returnValue'),
    enumerable: true,
    configurable: true,
  });
  g.__frot_ifaceattrs(Event.prototype, ['defaultPrevented', 'composed', 'timeStamp']);
  Object.defineProperty(Event.prototype, 'cancelBubble', {
    get: brand(function () {
      return !!slots(this).stop;
    }, 'get cancelBubble'),
    set: brand(function (v) {
      if (v) slots(this).stop = true;
    }, 'set cancelBubble'),
    enumerable: true,
    configurable: true,
  });
  // Gecko's two non-standard targets: the target itself wherever there is no
  // anonymous content, which frot never has.
  g.__frot_ifaceattrs(Event.prototype, { originalTarget: target, explicitOriginalTarget: target });
  var CONSTS = {
    NONE: 0, CAPTURING_PHASE: 1, AT_TARGET: 2, BUBBLING_PHASE: 3,
    ALT_MASK: 1, CONTROL_MASK: 2, SHIFT_MASK: 4, META_MASK: 8,
  };
  [Event.prototype, Event].forEach(function (o) {
    Object.keys(CONSTS).forEach(function (k) {
      Object.defineProperty(o, k, { value: CONSTS[k], enumerable: true });
    });
  });

  // CustomEvent.prototype: initCustomEvent, detail, constructor — parent Event.
  var CustomEvent = g.__frot_iface('CustomEvent', null, function (inst, args) {
    argc('CustomEvent constructor', 1, args.length);
    initEvent(inst, args[0], args[1]);
    slots(inst).detail = args[1] && args[1].detail !== undefined ? args[1].detail : null;
  });
  g.__frot_extends(CustomEvent, Event);
  g.__frot_ifaceops(CustomEvent.prototype, {
    initCustomEvent: function initCustomEvent(type, bubbles, cancelable, detail) {
      argc('CustomEvent.initCustomEvent', 1, arguments.length);
      Event.prototype.initEvent.call(this, type, bubbles, cancelable);
      if (!slots(this).dispatching) slots(this).detail = detail === undefined ? null : detail;
    },
  });
  g.__frot_ifaceattrs(CustomEvent.prototype, ['detail']);

  // ErrorEvent: what "report the exception" fires at the window (events.js).
  var ErrorEvent = g.__frot_iface('ErrorEvent', null, function (inst, args) {
    argc('ErrorEvent constructor', 1, args.length);
    var init = args[1] || {};
    initEvent(inst, args[0], init);
    Object.assign(slots(inst), {
      message: init.message === undefined ? '' : String(init.message),
      filename: init.filename === undefined ? '' : String(init.filename),
      lineno: init.lineno >>> 0,
      colno: init.colno >>> 0,
      error: init.error === undefined ? null : init.error,
    });
  });
  g.__frot_extends(ErrorEvent, Event);
  g.__frot_ifaceattrs(ErrorEvent.prototype, ['message', 'filename', 'lineno', 'colno', 'error']);

  // document.createEvent's interface table (doc.js), by lowercased name; an
  // event it makes is uninitialized. Only the interfaces frot builds.
  var CREATE = { event: Event, events: Event, htmlevents: Event, svgevents: Event, customevent: CustomEvent };
  Object.defineProperty(g, '__frot_create_event', {
    value: brand(function (name) {
      var C = CREATE[String(name).toLowerCase()];
      if (!C) throw g.__frot_domerror('NotSupportedError', 'Operation is not supported');
      var e = Object.create(C.prototype);
      initEvent(e, '', null, false);
      if (C === CustomEvent) slots(e).detail = null;
      return e;
    }, '__frot_create_event'),
    configurable: true,
  });
})(globalThis);
