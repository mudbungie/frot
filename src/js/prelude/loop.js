// Event loop (js.md §5) — virtual clock, timers, rAF, and the lifecycle events.
// The queue and the clock are private state of this realm (per-run, no global):
// the host drives macrotasks one at a time through __frot_next_timer / __frot_fire,
// draining microtasks between each (the engine's job queue). Loads after dom.js
// (needs Node/document), env.js, and events.js — WHO listens is that file's one
// registry and EventTarget interface; this file owns only WHEN the host fires,
// reaching it through the `__frot_dispatch` it exports. Augments the global
// (= window, §1 spike).
(function (g) {
  'use strict';

  // VIRTUAL_HORIZON_MS: a task due past it is dropped — "no timers past load"
  // (§5). rAF is a 16 ms virtual timer. Sub-ms delays clamp to 1 ms so every
  // timer advances the clock, guaranteeing horizon termination (§5).
  var HORIZON = 10000;
  var RAF_MS = 16;

  // --- One coherent browser clock (§5, bl-e707) -----------------------------
  // timeOrigin is the wall-clock ms at session start; performance.now() and
  // Date.now() BOTH read observe(), so the three derive from one origin.
  // observe() = the ONE injectable monotonic clock's real elapsed (host/CPU/
  // network time, via __frot_now) PLUS the virtual offset timer jumps add,
  // floored to the profile's timer precision (identity.md §9 — Firefox clamps to
  // 1 ms) and clamped so it never runs backward. __frot_now reads the SAME clock
  // the §5 deadline bounds JS/subfetch with, so a fetch's real wall time is
  // already in the reading — before/after a fetch differ by its duration, and a
  // synchronous/subfetch probe never sees perpetual zero.
  var PRECISION = JSON.parse(__frot_env_profile()).timerPrecisionUs / 1000;
  var origin = Date.now();
  var virtual = 0; // ms the virtual clock has jumped past real time (timer dues)
  var ratchet = 0; // last observed value — the never-backward guard
  function observe() {
    var t = Math.floor((__frot_now() + virtual) / PRECISION) * PRECISION;
    if (t < ratchet) t = ratchet;
    ratchet = t;
    return t;
  }
  Date.now = function () {
    return origin + observe();
  };
  // `performance` is a WebIDL interface, not an object literal: measured on
  // Firefox 153.0esr (identity.md §3.17) its instance owns nothing, its parent
  // is EventTarget, and its prototype lists the operations first — now, toJSON,
  // getEntries, getEntriesByType, getEntriesByName, … — with `timeOrigin` among
  // the attributes after them. frot publishes the subset it can answer, in that
  // measured relative order (bl-643d).
  var Performance = g.__frot_iface('Performance');
  Object.setPrototypeOf(Performance.prototype, g.EventTarget.prototype);
  g.__frot_ifaceops(Performance.prototype, {
    now: function now() {
      return observe();
    },
    getEntries: function getEntries() {
      return resourceEntries();
    },
    getEntriesByType: function getEntriesByType(type) {
      return type === 'resource' ? resourceEntries() : [];
    },
    getEntriesByName: function getEntriesByName(name) {
      return resourceEntries().filter(function (e) {
        return e.name === name;
      });
    },
  });
  g.__frot_ifaceattrs(Performance.prototype, {
    timeOrigin: function () {
      return origin;
    },
  });
  g.performance = Object.create(Performance.prototype);
  // PerformanceResourceTiming for the real subfetches only (§6/§8): frot measures
  // start + duration off the one clock; every phase it does not measure (DNS/TCP/
  // TLS) stays 0 — spec-legal for a cross-origin resource, never fabricated. No
  // navigation entry is synthesised for the pre-JS document fetch (unmeasured
  // here) — spec-legal omission over invented phases.
  function resourceEntries() {
    return __frot_resource_timings().map(function (r) {
      var s = Math.floor(r[1] / PRECISION) * PRECISION;
      var d = Math.floor(r[2] / PRECISION) * PRECISION;
      // Measured: startTime/duration/responseEnd. Unmeasured phases spec-legal 0.
      return {
        entryType: 'resource', name: r[0], startTime: s, duration: d,
        fetchStart: s, responseEnd: s + d,
        domainLookupStart: 0, domainLookupEnd: 0, connectStart: 0, connectEnd: 0,
        secureConnectionStart: 0, requestStart: 0, responseStart: 0, transferSize: 0,
      };
    });
  }

  // --- Timer queue ----------------------------------------------------------
  var timers = [];
  var nextId = 1;
  function schedule(cb, delay, args, interval) {
    if (typeof cb !== 'function') return 0;
    delay = +delay;
    delay = delay > 1 ? delay : 1;
    var id = nextId++;
    timers.push({ id: id, due: observe() + delay, cb: cb, args: args, interval: interval });
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
        cb(observe());
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
    // Jump the observable clock forward to at least the timer's due time — the
    // virtual offset (§5). Timers still fire immediately in host wall time; only
    // the *reading* jumps, so performance.now() >= the scheduled delay.
    var c = observe();
    if (t.due > c) virtual += t.due - c;
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

  // --- Host-fired lifecycle (§4.4 / §5) ---------------------------------------
  // The listener registry and the EventTarget interface are events.js's; this is
  // the only place the HOST fires into them, through its one exported dispatch.
  var dispatch = g.__frot_dispatch;

  // readyState transition (js.md §4.4): the field advances loading ->
  // 'interactive' (immediately before DOMContentLoaded) -> 'complete' (before
  // load); each transition dispatches readystatechange on document so
  // readyState-polling init and older loaders unblock. Returns the listener
  // throw count (folded into the loop's error tally like any handler).
  function setReadyState(state) {
    g.document.readyState = state;
    var ev = new g.Event('readystatechange');
    var errs = dispatch('document', ev, g.document);
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
    errs += dispatch('document', ev, g.document) + dispatch('window', ev, g);
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
    dispatch('window', ev, g);
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
