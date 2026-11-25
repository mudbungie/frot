// Events: the ONE listener registry, the EventTarget interface over it, and the
// ONE dispatch path every event goes through (bl-e81b, bl-6438, bl-810b; js.md
// §4.4/§11, identity.md §10). Whoever raises an event — a page's
// `dispatchEvent`, the host lifecycle (loop.js), an <img>'s load/error
// (imageload.js), a reader's progress (filereader.js) — hands it to `dispatch`,
// which runs the DOM standard's algorithm (§2.9 "Dispatching events") as
// Firefox 153.0esr does it, measured (src/js/tests/dispatch/firefox153.json):
// the path from the target up the tree to the document and on to the window
// (never the window for `load`; across shadow boundaries as event.js
// builds it), a capture pass down it, the target's own
// listeners (capture ones first), and — for an event that bubbles — a bubble
// pass back up; `stopPropagation` ends the walk after the current object,
// `stopImmediatePropagation` at once. A listener that throws is REPORTED (DOM
// "inner invoke" step 10): the exception becomes an ErrorEvent at the window,
// then `window.onerror`, and counts toward js.errors unless one of them
// handles it (js.md §10) — the next listener runs regardless.
// Runs after brand.js, dom.js and event.js; BEFORE loop.js, permissions.js,
// worker.js and idb.js, which all reach it.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;

  // The one registry: target object -> type -> listener entries, each the
  // standard's event listener struct ({callback, capture, once, passive,
  // removed}). A WeakMap, so it is never a property a page can walk and a
  // dropped target takes its listeners with it.
  var lists = new WeakMap();
  function listeners(target, type) {
    var t = lists.get(target);
    if (!t) lists.set(target, (t = Object.create(null)));
    return t[type] || (t[type] = []);
  }
  function find(list, callback, capture) {
    for (var i = 0; i < list.length; i++)
      if (list[i].callback === callback && list[i].capture === capture) return i;
    return -1;
  }
  function remove(target, type, callback, capture) {
    var list = listeners(target, type);
    var i = find(list, callback, capture);
    if (i >= 0) list.splice(i, 1)[0].removed = true;
  }

  function argc(op, need, got) {
    if (got < need)
      throw new TypeError('EventTarget.' + op + ': At least ' + need + ' argument'
        + (need > 1 ? 's' : '') + ' required, but only ' + got + ' passed');
  }
  function isObject(v) {
    return v !== null && (typeof v === 'object' || typeof v === 'function');
  }
  // A call with no receiver is a call on the window (`bl-a7c5`): strict code
  // that calls a bare `addEventListener('message', f)` passes `this ===
  // undefined`, and WebIDL resolves that to the global for an operation the
  // global object inherits. Cloudflare's challenge script does exactly this.
  function receiver(o) {
    return o == null ? g : o;
  }
  // The options argument: a boolean is `capture`; any object is the
  // AddEventListenerOptions dictionary, read in its member order.
  function options(o, full) {
    if (!isObject(o)) return { capture: !!o };
    var r = { capture: !!o.capture };
    if (!full) return r;
    r.once = !!o.once;
    r.passive = !!o.passive;
    r.signal = o.signal;
    if (r.signal !== undefined && !isObject(r.signal))
      throw new TypeError("EventTarget.addEventListener: 'signal' member of AddEventListenerOptions is not an object.");
    if (r.signal !== undefined && !(r.signal instanceof g.AbortSignal))
      throw new TypeError("EventTarget.addEventListener: 'signal' member of AddEventListenerOptions does not implement interface AbortSignal.");
    return r;
  }

  g.EventTarget = function EventTarget() {
    g.__frot_new(new.target, 'EventTarget');
  };
  var ET = g.EventTarget.prototype;
  ET.addEventListener = function (type, callback) {
    var opts = arguments[2];
    argc('addEventListener', 2, arguments.length);
    var target = receiver(this);
    if (callback == null) return;
    if (!isObject(callback)) throw new TypeError('EventTarget.addEventListener: Argument 2 is not an object.');
    var o = options(opts, true);
    if (o.signal && o.signal.aborted) return;
    type = String(type);
    var list = listeners(target, type);
    if (find(list, callback, o.capture) >= 0) return;
    list.push({ callback: callback, capture: o.capture, once: o.once, passive: o.passive, removed: false });
    if (o.signal)
      o.signal.addEventListener('abort', function () {
        remove(target, type, callback, o.capture);
      });
  };
  ET.removeEventListener = function (type, callback) {
    argc('removeEventListener', 2, arguments.length);
    remove(receiver(this), String(type), callback, options(arguments[2]).capture);
  };
  ET.dispatchEvent = function (ev) {
    argc('dispatchEvent', 1, arguments.length);
    if (!isObject(ev)) throw new TypeError('EventTarget.dispatchEvent: Argument 1 is not an object.');
    if (!slots(ev).event)
      throw new TypeError('EventTarget.dispatchEvent: Argument 1 does not implement interface Event.');
    return dispatch(receiver(this), ev);
  };
  Object.defineProperty(ET, Symbol.toStringTag, { value: 'EventTarget', configurable: true });
  Object.defineProperty(g.EventTarget, Symbol.hasInstance, {
    // The shape test is EventTarget's own answer (framer-motion resolves
    // animation targets with `t instanceof EventTarget`, and the window is not
    // one by inheritance here); an interface that inherits it (Node and
    // everything under it) answers by its prototype chain.
    value: function (o) {
      if (this !== g.EventTarget) return Function.prototype[Symbol.hasInstance].call(this, o);
      return isObject(o) && typeof o.addEventListener === 'function';
    },
  });
  // The window takes the trio as its own (it is not an EventTarget by
  // inheritance here); the document inherits it through Node (bl-9f79), and
  // so does every node (bl-78cb). Branding at the definition is what makes
  // them read native everywhere they are reached.
  ['addEventListener', 'removeEventListener', 'dispatchEvent'].forEach(function (k) {
    brand(ET[k], k);
    g[k] = ET[k];
  });
  g.__frot_extends(g.Node, g.EventTarget);

  // Report an exception (HTML "report the exception"): an ErrorEvent at the window, through
  // the path like any event, then `window.onerror`; counted unless one of them
  // handled it. An exception thrown while one is being reported is counted
  // bare, never reported again (measured: no second `error` event).
  var reporting = false;
  function report(e) {
    var message = e instanceof Error ? String(e) : 'uncaught exception: ' + String(e);
    if (reporting) return g.__frot_report_error(message);
    var ev = new g.ErrorEvent('error', { cancelable: true, message: message, error: e });
    slots(ev).trusted = true;
    raise(ev);
  }
  // A window `error` event, as the error accounting sees it (js.md §10):
  // dispatched — `window.onerror` is one of its listeners (handlers.js), and
  // its `true` cancels it — and, not canceled, counted toward js.errors. React
  // ≥16 reports caught render errors through `reportError`, which lands here.
  function raise(ev) {
    var was = reporting;
    reporting = true;
    try {
      var ok = dispatch(g, ev);
      if (ok) g.__frot_report_error(ev.message != null ? String(ev.message) : '');
      return ok;
    } finally {
      reporting = was;
    }
  }

  // Inner invoke (DOM §2.9): `object`'s listeners for this pass, cloned first
  // so a listener added during dispatch waits for the next one.
  function invoke(entry, ev, st, phase, capturePass) {
    var object = entry.item;
    st.target = entry.target;
    st.currentTarget = object;
    st.eventPhase = phase;
    g.__frot_handler_discover(object, st.type);
    var list = listeners(object, st.type).slice();
    for (var i = 0; i < list.length && !st.stopNow; i++) {
      var l = list[i];
      if (l.removed || l.capture !== capturePass) continue;
      if (l.once) remove(object, st.type, l.callback, l.capture);
      st.passive = l.passive;
      try {
        var cb = l.callback;
        if (typeof cb === 'function') cb.call(object, ev);
        else {
          var h = cb.handleEvent;
          if (typeof h !== 'function') throw new TypeError("Property 'handleEvent' is not callable.");
          h.call(cb, ev);
        }
      } catch (e) {
        report(e);
      }
      st.passive = false;
    }
  }

  // Dispatch (DOM §2.9): `target` is where the event goes; `as`, when given,
  // is what `event.target` reads instead — the "legacy target override" the
  // window's `load` uses, whose target is the document though its path is
  // the window alone. Returns false when a listener canceled it.
  function dispatch(target, ev, as) {
    var st = slots(ev);
    if (st.dispatching || !st.initialized)
      throw g.__frot_domerror('InvalidStateError', 'An attempt was made to use an object that is not, or is no longer, usable');
    st.dispatching = true;
    // The path (event.js): each object with the target it sees, at its
    // target where that is itself — the first, or a shadow host.
    var path = as ? [{ item: target, target: as }] : g.__frot_event_path.path(target, st);
    var at = function (k) {
      return !k || path[k].item === path[k].target;
    };
    st.path = path;
    try {
      for (var i = path.length - 1; i >= 0 && !st.stop; i--) invoke(path[i], ev, st, at(i) ? 2 : 1, true);
      for (var j = 0; j < path.length && !st.stop; j++)
        if (at(j) || st.bubbles) invoke(path[j], ev, st, at(j) ? 2 : 3, false);
    } finally {
      st.target = as || g.__frot_event_path.settled(path);
      st.eventPhase = 0;
      st.currentTarget = null;
      st.path = [];
      st.dispatching = st.stop = st.stopNow = false;
    }
    return !st.defaultPrevented;
  }

  // The path, for the host (loop.js, imageload.js, filereader.js); the
  // reporting, for loop.js's `reportError` and a page-dispatched window
  // `error`. Non-enumerable like every `__frot_*` helper, branded so their
  // source never shows.
  // The registry itself, for handlers.js: an event handler is a listener
  // entry it places (at the end, or first for a parsed attribute) and takes
  // back, never one a page can name to removeEventListener.
  var registry = {
    add: function (target, type, callback, first) {
      var e = { callback: callback, capture: false, once: false, passive: false, removed: false };
      var list = listeners(target, type);
      if (first) list.unshift(e);
      else list.push(e);
      return e;
    },
    remove: function (target, type, e) {
      var list = listeners(target, type), i = list.indexOf(e);
      if (i >= 0) list.splice(i, 1);
      e.removed = true;
    },
    // A node adopted from another window's document takes its listeners
    // along (adopt.js): released from that registry, adopted into this one.
    release: function (target) {
      var t = lists.get(target);
      lists.delete(target);
      return t;
    },
    adopt: function (target, t) {
      if (t) lists.set(target, t);
    },
  };
  Object.defineProperty(g, '__frot_registry', { value: registry, configurable: true });
  [['__frot_dispatch', dispatch], ['__frot_report_exception', report], ['__frot_raise_error', raise]].forEach(function (p) {
    Object.defineProperty(g, p[0], { value: brand(p[1], p[0]), configurable: true, writable: true });
  });
})(globalThis);
