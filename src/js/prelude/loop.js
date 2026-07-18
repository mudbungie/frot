// Event loop (js.md §5) — virtual clock, timers, rAF, and the lifecycle events.
// The queue and the clock are private state of this realm (per-run, no global):
// the host drives macrotasks one at a time through __frot_next_timer / __frot_fire,
// draining microtasks between each (the engine's job queue). Loads after dom.js
// (needs Node/document) and env.js. Augments the global (= window, §1 spike).
(function (g) {
  'use strict';

  // VIRTUAL_HORIZON_MS: a task due past it is dropped — "no timers past load"
  // (§5). rAF is a 16 ms virtual timer. Sub-ms delays clamp to 1 ms so every
  // timer advances the clock, guaranteeing horizon termination (§5).
  var HORIZON = 10000;
  var RAF_MS = 16;

  // --- Virtual clock: seeded from real time at start, advanced by the loop ---
  // Date.now/performance.now read virtual time (§5). The engine's native
  // performance.now is a locked property, so replace the whole object.
  var origin = Date.now();
  var clock = 0;
  Date.now = function () {
    return origin + clock;
  };
  g.performance = {
    now: function () {
      return clock;
    },
    timeOrigin: origin,
  };

  // --- Timer queue ----------------------------------------------------------
  var timers = [];
  var nextId = 1;
  function schedule(cb, delay, args, interval) {
    if (typeof cb !== 'function') return 0;
    delay = +delay;
    delay = delay > 1 ? delay : 1;
    var id = nextId++;
    timers.push({ id: id, due: clock + delay, cb: cb, args: args, interval: interval });
    return id;
  }
  function clear(id) {
    for (var i = 0; i < timers.length; i++)
      if (timers[i].id === id) {
        timers.splice(i, 1);
        return;
      }
  }
  function rest(from) {
    return Array.prototype.slice.call(from, 2);
  }
  g.setTimeout = function (cb, delay) {
    return schedule(cb, delay, rest(arguments), 0);
  };
  g.setInterval = function (cb, delay) {
    delay = +delay;
    delay = delay > 1 ? delay : 1;
    return schedule(cb, delay, rest(arguments), delay);
  };
  g.clearTimeout = clear;
  g.clearInterval = clear;
  g.requestAnimationFrame = function (cb) {
    return schedule(
      function () {
        cb(clock);
      },
      RAF_MS,
      [],
      0
    );
  };
  g.cancelAnimationFrame = clear;

  // Host driver: run the earliest timer due within the horizon, jumping the
  // clock to it; reschedule intervals (dropping any that step past the horizon).
  // Returns -1 when nothing is due (the loop is settled), else the callback's
  // error count (0 or 1) — a throw is counted and the loop continues (§5).
  g.__frot_next_timer = function () {
    var best = -1;
    for (var i = 0; i < timers.length; i++)
      if (timers[i].due <= HORIZON && (best < 0 || timers[i].due < timers[best].due)) best = i;
    if (best < 0) return -1;
    var t = timers[best];
    clock = t.due;
    if (t.interval > 0) {
      t.due += t.interval;
      if (t.due > HORIZON) timers.splice(best, 1);
    } else {
      timers.splice(best, 1);
    }
    try {
      t.cb.apply(g, t.args);
      return 0;
    } catch (e) {
      return 1;
    }
  };

  // --- Events: DOMContentLoaded/load are the only host-fired events (§11);
  // addEventListener elsewhere registers handlers that fire only if the page
  // dispatches them itself (no interaction, §11). Listeners key by target so the
  // stateless Node wrappers (fresh per syscall) never hold them.
  var reg = Object.create(null);
  function bucket(key, type) {
    var t = reg[key] || (reg[key] = Object.create(null));
    return t[type] || (t[type] = []);
  }
  function add(key, type, fn) {
    if (typeof fn !== 'function') return;
    var b = bucket(key, type);
    if (b.indexOf(fn) < 0) b.push(fn);
  }
  function remove(key, type, fn) {
    var b = bucket(key, type);
    var i = b.indexOf(fn);
    if (i >= 0) b.splice(i, 1);
  }
  function fire(key, ev, target) {
    var b = bucket(key, ev.type).slice();
    var errs = 0;
    for (var i = 0; i < b.length; i++)
      try {
        b[i].call(target, ev);
      } catch (e) {
        errs++;
      }
    return errs;
  }
  function bindEvents(o, key, target) {
    o.addEventListener = function (type, fn) {
      add(key, String(type), fn);
    };
    o.removeEventListener = function (type, fn) {
      remove(key, String(type), fn);
    };
    o.dispatchEvent = function (ev) {
      fire(key, ev, target);
      return !(ev && ev.defaultPrevented);
    };
  }

  g.Event = function (type, init) {
    this.type = String(type);
    this.bubbles = !!(init && init.bubbles);
    this.defaultPrevented = false;
    this.target = null;
  };
  g.Event.prototype.preventDefault = function () {
    this.defaultPrevented = true;
  };
  g.Event.prototype.stopPropagation = function () {};
  g.CustomEvent = function (type, init) {
    g.Event.call(this, type, init);
    this.detail = init ? init.detail : null;
  };
  g.CustomEvent.prototype = Object.create(g.Event.prototype);

  bindEvents(g, 'window', g);
  bindEvents(g.document, 'document', g.document);
  g.Node.prototype.addEventListener = function (type, fn) {
    add('n' + this._id, String(type), fn);
  };
  g.Node.prototype.removeEventListener = function (type, fn) {
    remove('n' + this._id, String(type), fn);
  };
  g.Node.prototype.dispatchEvent = function (ev) {
    fire('n' + this._id, ev, this);
    return !(ev && ev.defaultPrevented);
  };

  // readyState transition (js.md §4.4): the field advances loading ->
  // 'interactive' (immediately before DOMContentLoaded) -> 'complete' (before
  // load); each transition dispatches readystatechange on document so
  // readyState-polling init and older loaders unblock. Returns the listener
  // throw count (folded into the loop's error tally like any handler).
  function setReadyState(state) {
    g.document.readyState = state;
    var ev = new g.Event('readystatechange');
    ev.target = g.document;
    var errs = fire('document', ev, g.document);
    if (typeof g.document.onreadystatechange === 'function')
      try {
        g.document.onreadystatechange.call(g.document, ev);
      } catch (e) {
        errs++;
      }
    return errs;
  }

  // Host lifecycle fire (§4.4 / §5): advance readyState (before the matching
  // event), dispatch to document + window and the matching on<event> property
  // handler, returning the listener error count.
  g.__frot_fire = function (name) {
    var errs = 0;
    if (name === 'DOMContentLoaded') errs += setReadyState('interactive');
    else if (name === 'load') errs += setReadyState('complete');
    var ev = new g.Event(name);
    ev.target = g.document;
    errs += fire('document', ev, g.document) + fire('window', ev, g);
    var on = 'on' + name.toLowerCase();
    var hosts = [g.document, g];
    for (var i = 0; i < hosts.length; i++)
      if (typeof hosts[i][on] === 'function')
        try {
          hosts[i][on].call(hosts[i], ev);
        } catch (e) {
          errs++;
        }
    return errs;
  };

  // --- Unhandled-error accounting (js.md §10) --------------------------------
  // React >=16 CATCHES render errors and reports them via reportError (else
  // console.error) rather than throwing, so a dead app would otherwise count 0
  // errors. reportError and a directly dispatched window 'error' event funnel
  // through raiseError: fire the window 'error' listeners and window.onerror;
  // if nothing suppresses it (no preventDefault, onerror didn't return true) it
  // is an UNHANDLED error and is counted into js.errors via __frot_report_error.
  function raiseError(ev) {
    fire('window', ev, g);
    var suppressed = ev.defaultPrevented;
    if (typeof g.onerror === 'function') {
      var handled;
      try {
        handled = g.onerror(ev.message, '', 0, 0, ev.error);
      } catch (e) {
        handled = false;
      }
      if (handled === true) suppressed = true;
    }
    if (!suppressed) g.__frot_report_error(ev.message != null ? String(ev.message) : '');
    return !ev.defaultPrevented;
  }
  g.reportError = function (err) {
    var ev = new g.Event('error');
    ev.target = g;
    ev.error = err;
    ev.message = err && err.message != null ? String(err.message) : String(err);
    raiseError(ev);
  };
  // Route a directly dispatched window 'error' event through the same
  // accounting; every other event type keeps the plain window dispatch.
  var rawWindowDispatch = g.dispatchEvent;
  g.dispatchEvent = function (ev) {
    if (ev && ev.type === 'error') return raiseError(ev);
    return rawWindowDispatch(ev);
  };
})(globalThis);
