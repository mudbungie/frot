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
  // One agent, one loop (js.md §14): an iframe's window is handed its creator's
  // loop state as `__frot_agent` and schedules into that one queue on that one
  // clock; only the top window makes a new one. A window's own time origin is
  // `base` on the agent's clock — 0 for the top — so performance.now() is
  // per-window while Date.now() is the agent's, as in a browser.
  var agent = g.__frot_agent;
  delete g.__frot_agent;
  var shared = agent || { timers: [], nextId: 1, virtual: 0, ratchet: 0, origin: Date.now() };
  // virtual: ms the virtual clock has jumped past real time (timer dues);
  // ratchet: the last observed value — the never-backward guard.
  if (!agent)
    shared.observe = function () {
      var t = Math.floor((__frot_now() + shared.virtual) / PRECISION) * PRECISION;
      if (t < shared.ratchet) t = shared.ratchet;
      shared.ratchet = t;
      return t;
    };
  var base = agent ? shared.observe() : 0;
  var origin = shared.origin + base;
  function observe() {
    return shared.observe() - base;
  }
  // An event's timeStamp is this window's performance.now(), read without
  // going through the page-visible (and probe-counted) `performance`.
  Object.defineProperty(g, '__frot_event_time', { value: observe, configurable: true });
  Date.now = function () {
    return shared.origin + shared.observe();
  };
  // frame.js hands `shared` to each window it opens; a frame's initial
  // about:blank document is complete at birth (measured, js.md §14).
  Object.defineProperty(g, '__frot_loop', { value: shared, configurable: true });
  // A task on this window's loop, FIFO with the zero-delay timers (message.js).
  Object.defineProperty(g, '__frot_task', {
    value: function (run) {
      schedule(run, 0, [], 0);
    },
    configurable: true,
  });
  if (agent) g.__frot_slots(g.document).readyState = 'complete';
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
      need(arguments, 'getEntriesByType');
      return String(type) === 'resource' ? resourceEntries() : [];
    },
    getEntriesByName: function getEntriesByName(name) {
      need(arguments, 'getEntriesByName');
      var type = arguments[1] === undefined ? undefined : String(arguments[1]);
      return resourceEntries().filter(function (e) {
        return e.name === String(name) && (type === undefined || type === e.entryType);
      });
    },
  });
  g.__frot_ifaceattrs(Performance.prototype, {
    timeOrigin: function () {
      return origin;
    },
  });
  g.performance = Object.create(Performance.prototype);
  function need(args, member) {
    if (!args.length) {
      throw new TypeError('Performance.' + member + ': At least 1 argument required, but only 0 '
        + 'passed');
    }
  }
  // One PerformanceResourceTiming per real subfetch (§6/§8, bl-c378), made once
  // and handed back as the same object at every read (measured). What an entry
  // says is perfobserver.js's (`perf.entry`); the times are floored to the
  // timer precision here. No navigation entry is synthesised for the pre-JS
  // document fetch (unmeasured here) — spec-legal omission over invented phases.
  var made = [];
  function resourceEntries() {
    var raw = __frot_resource_timings();
    for (var i = made.length; i < raw.length; i++) {
      made.push(perf.entry(raw[i], i, function (ms) {
        return Math.floor(ms / PRECISION) * PRECISION;
      }));
    }
    return made.slice();
  }

  // --- Timer queue ----------------------------------------------------------
  // Dues are on the agent's clock; each timer runs with its own window as `this`.
  var timers = shared.timers;
  function schedule(cb, delay, args, interval) {
    if (typeof cb !== 'function') return 0;
    delay = +delay;
    delay = delay > 1 ? delay : 1;
    var id = shared.nextId++;
    timers.push({ id: id, due: shared.observe() + delay, cb: cb, args: args, interval: interval, self: g });
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

  // --- The idle-callback seam (bl-b584) ----------------------------------------
  // idle.js builds requestIdleCallback on these four; it removes the global.
  // An idle callback is a timer 1 ms out, flagged so the driver below runs every
  // ordinary timer due at that instant first.
  Object.defineProperty(g, '__frot_idle_hook', {
    configurable: true,
    value: {
      observe: observe,
      queue: function (run, handle) {
        schedule(run, 1, [], 0);
        timers[timers.length - 1].idle = handle;
      },
      cancel: function (handle) {
        for (var i = timers.length - 1; i >= 0; i--) if (timers[i].idle === handle) timers.splice(i, 1);
      },
      // When the idle period that starts at `now` ends: 50 ms on, or sooner if a
      // timer is due first. `now` is this window's time; dues are the agent's.
      until: function (now) {
        var at = now + base;
        return timers.reduce(function (soonest, t) {
          return Math.min(soonest, Math.max(at, t.due));
        }, at + 50) - base;
      },
    },
  });

  // The PerformanceObserver seam (bl-b129): perfobserver.js delivers on this
  // queue, and sets `before` while an observer is active so new resource
  // timings are noticed whenever the driver asks for the next task.
  var perf = { schedule: schedule, entries: resourceEntries, before: null };
  Object.defineProperty(g, '__frot_perf_hook', { configurable: true, value: perf });

  // Host driver: run the earliest timer due within the horizon, jumping the
  // clock to it; reschedule intervals (dropping any that step past the horizon).
  // Returns -1 when nothing is due (the loop is settled), else the callback's
  // error count (0 or 1) — a throw is counted and the loop continues (§5).
  g.__frot_next_timer = function () {
    if (perf.before) perf.before();
    var best = -1;
    // Earliest due first; at the same instant an ordinary timer beats an idle
    // callback, and equals keep their queue order.
    function before(a, b) {
      return a.due < b.due || (a.due === b.due && !a.idle && !!b.idle);
    }
    for (var i = 0; i < timers.length; i++)
      if (timers[i].due <= HORIZON && (best < 0 || before(timers[i], timers[best]))) best = i;
    if (best < 0) return -1;
    var t = timers[best];
    // Jump the observable clock forward to at least the timer's due time — the
    // virtual offset (§5). Timers still fire immediately in host wall time; only
    // the *reading* jumps, so performance.now() >= the scheduled delay.
    var c = shared.observe();
    if (t.due > c) shared.virtual += t.due - c;
    if (t.interval > 0) {
      t.due += t.interval;
      if (t.due > HORIZON) timers.splice(best, 1);
    } else {
      timers.splice(best, 1);
    }
    // The entry window while the callback runs: whose task this is
    // (message.js reads it as a posted message's `source`).
    var was = shared.entry;
    shared.entry = t.self;
    try {
      t.cb.apply(t.self, t.args);
      return 0;
    } catch (e) {
      return 1;
    } finally {
      shared.entry = was;
    }
  };

  // --- Host-fired lifecycle (§4.4 / §5) ---------------------------------------
  // The listener registry, the dispatch path and the error reporting are
  // events.js's; this is only WHEN the host fires into them. A listener's throw
  // is reported (and counted) on that path, so these drivers return 0 for it.
  var dispatch = g.__frot_dispatch;
  function trusted(type, init) {
    var ev = new g.Event(type, init);
    g.__frot_slots(ev).trusted = true;
    return ev;
  }
  // readyState transition (js.md §4.4): the field advances loading ->
  // 'interactive' (immediately before DOMContentLoaded) -> 'complete' (before
  // load); each transition fires readystatechange at the document (it does not
  // bubble), so readyState-polling init and older loaders unblock.
  function setReadyState(state) {
    g.__frot_slots(g.document).readyState = state;
    dispatch(g.document, trusted('readystatechange'));
  }

  // Host lifecycle fire (§4.4 / §5), as HTML fires them: DOMContentLoaded at
  // the document, bubbling (so the window hears it on the way up); `load` at
  // the window with the document as its target (the legacy target override).
  g.__frot_fire = function (name) {
    if (name === 'DOMContentLoaded') {
      setReadyState('interactive');
      dispatch(g.document, trusted(name, { bubbles: true }));
      g.__frot_autofocus(); // after the handlers: they still see <body> (bl-4248)
    } else {
      setReadyState('complete');
      dispatch(g, trusted(name), g.document);
    }
    return 0;
  };

  // --- Unhandled-error accounting (js.md §10) --------------------------------
  // React >=16 CATCHES render errors and reports them via reportError (else
  // console.error) rather than throwing, so a dead app would otherwise count 0
  // errors. reportError reports its argument like an uncaught exception, and a
  // page-dispatched window 'error' event goes through the same accounting
  // (events.js `raise`): unhandled, it counts toward js.errors.
  g.reportError = function (err) {
    g.__frot_report_exception(err);
  };
  var rawWindowDispatch = g.dispatchEvent;
  g.dispatchEvent = function (ev) {
    if (g.__frot_slots(ev).event && ev.type === 'error') return g.__frot_raise_error(ev);
    return rawWindowDispatch.apply(this, arguments);
  };
})(globalThis);
