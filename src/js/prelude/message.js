// Messaging (bl-0af7): window.postMessage, MessageEvent, MessageChannel /
// MessagePort and BroadcastChannel, measured on Firefox 153.0esr
// (src/js/tests/message/firefox153.json). Every delivery is a task on the one
// agent loop (js.md §5, §14 step 5) — FIFO with the zero-delay timers, as the
// binary orders them — so nothing here outlives the run and nothing new needs
// driving. The data is structured-cloned at the post. Channels and
// BroadcastChannel subscriptions live in the agent's loop state, shared by
// every window of the run and gone with it. `event.source` is the window whose
// task was running when the post was made (the entry window): exact for a post
// from a timer, a listener or a script of that window, where the binary's
// incumbent is the caller's realm — a frame's code run synchronously by its
// parent's `w.eval` reads as the parent's (js.md §3).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;
  var shared = g.__frot_loop;
  var domError = g.__frot_domerror;

  // --- Structured clone (HTML §2.7) over the values a page posts ------------
  var TYPED = ['Int8Array', 'Uint8Array', 'Uint8ClampedArray', 'Int16Array', 'Uint16Array',
    'Int32Array', 'Uint32Array', 'Float32Array', 'Float64Array', 'BigInt64Array', 'BigUint64Array',
    'DataView'];
  function cloneErr(m) {
    return domError('DataCloneError', m);
  }
  function clone(v, seen) {
    if (typeof v === 'function') throw cloneErr('Function object could not be cloned.');
    if (typeof v === 'symbol') throw cloneErr('unsupported type for structured data');
    if (v === null || typeof v !== 'object') return v;
    if (seen.has(v)) return seen.get(v);
    var tag = Object.prototype.toString.call(v).slice(8, -1), out;
    if (Array.isArray(v)) out = [];
    else if (tag === 'Date') out = new Date(v.getTime());
    else if (tag === 'RegExp') out = new RegExp(v.source, v.flags);
    else if (tag === 'ArrayBuffer') out = v.slice(0);
    else if (TYPED.indexOf(tag) >= 0) out = new g[tag](clone(v.buffer, seen), v.byteOffset, tag === 'DataView' ? v.byteLength : v.length);
    else if (tag === 'Map') out = new Map();
    else if (tag === 'Set') out = new Set();
    else if (tag === 'Boolean' || tag === 'Number' || tag === 'String') out = Object(v.valueOf());
    else if (v instanceof Error) out = new (g[v.name] || Error)(v.message);
    else if (tag === 'Object' && !(v instanceof g.EventTarget)) out = {};
    else throw cloneErr('The object could not be cloned.');
    seen.set(v, out);
    if (tag === 'Map') v.forEach(function (val, k) { out.set(clone(k, seen), clone(val, seen)); });
    else if (tag === 'Set') v.forEach(function (val) { out.add(clone(val, seen)); });
    else if (Array.isArray(v) || tag === 'Object')
      Object.keys(v).forEach(function (k) {
        out[k] = clone(v[k], seen);
      });
    return out;
  }

  // The window's own structuredClone (bl-d8f7), over the same algorithm.
  g.structuredClone = g.__frot_brand(function structuredClone(value) {
    if (!arguments.length) {
      throw new TypeError('structuredClone: At least 1 argument required, but only 0 passed');
    }
    return clone(value, new Map());
  }, 'structuredClone');

  // --- MessageEvent ----------------------------------------------------------
  function isPort(p) {
    return p instanceof g.MessagePort;
  }
  function frozen(ports) {
    return Object.freeze(Array.prototype.slice.call(ports || []));
  }
  var MessageEvent = g.__frot_iface('MessageEvent', null, function (inst, args) {
    if (!args.length)
      throw new TypeError('MessageEvent constructor: At least 1 argument required, but only 0 passed');
    var init = args[1] || {};
    var source = init.source === undefined ? null : init.source;
    if (source !== null && !isPort(source) && !(source && source.window === source))
      throw new TypeError("MessageEvent constructor: 'source' member of MessageEventInit could not be "
        + 'converted to any of: WindowProxy, MessagePort, ServiceWorker.');
    g.__frot_event_init(inst, args[0], init);
    Object.assign(slots(inst), {
      data: init.data === undefined ? null : init.data,
      origin: init.origin === undefined ? '' : String(init.origin),
      lastEventId: init.lastEventId === undefined ? '' : String(init.lastEventId),
      source: source,
      ports: frozen(init.ports),
    });
  });
  g.__frot_extends(MessageEvent, g.Event);
  g.__frot_ifaceops(MessageEvent.prototype, {
    initMessageEvent: function initMessageEvent(type, bubbles, cancelable, data, origin, lastEventId, source, ports) {
      g.Event.prototype.initEvent.call(this, type, bubbles, cancelable);
      Object.assign(slots(this), {
        data: data === undefined ? null : data,
        origin: origin === undefined ? '' : String(origin),
        lastEventId: lastEventId === undefined ? '' : String(lastEventId),
        source: source === undefined ? null : source,
        ports: frozen(ports),
      });
    },
  });
  g.__frot_ifaceattrs(MessageEvent.prototype, ['data', 'origin', 'lastEventId', 'source', 'ports']);

  // Deliver a trusted `message` at `target` on a task of `win`'s loop, through
  // `win`'s own dispatch (each window's listeners are its realm's).
  function deliver(win, target, init, ok) {
    win.__frot_task(function () {
      if (ok && !ok()) return;
      var ev = new win.MessageEvent('message', init);
      win.__frot_slots(ev).trusted = true;
      win.__frot_dispatch(target, ev);
    });
  }
  function origin() {
    return g.top.location.origin;
  }

  // --- MessagePort / MessageChannel -----------------------------------------
  // A port: its entangled partner, whether it has been started (an `onmessage`
  // set starts it), its queue while not, and whether it is closed or was
  // transferred away (both make a post a silent no-op, measured).
  var Port = g.MessagePort;
  function makePort() {
    var p = Object.create(Port.prototype);
    Object.assign(slots(p), { port: true, partner: null, started: false, queue: [], dead: false });
    return p;
  }
  function entangle(a, b) {
    slots(a).partner = b;
    slots(b).partner = a;
  }
  function enqueue(port, init) {
    var st = slots(port);
    if (st.started) deliver(g, port, init, function () { return !st.dead; });
    else st.queue.push(init);
  }
  function transfer(list, self) {
    var seen = [];
    return Array.prototype.map.call(list || [], function (p) {
      if (!isPort(p) || p === self || seen.indexOf(p) >= 0 || slots(p).dead)
        throw cloneErr('The object could not be cloned.');
      seen.push(p);
      var moved = makePort(), partner = slots(p).partner;
      if (partner) entangle(moved, partner);
      slots(moved).queue = slots(p).queue;
      slots(p).dead = true;
      return moved;
    });
  }
  function options(o) {
    return Array.isArray(o) || o == null ? { transfer: o } : o;
  }
  g.__frot_ifaceops(Port.prototype, {
    postMessage: function postMessage(message) {
      var st = slots(this), o = options(arguments[1]);
      var data = clone(message, new Map()), ports = transfer(o.transfer, this);
      if (st.dead || !st.partner) return;
      enqueue(st.partner, { data: data, ports: ports });
    },
    start: function start() {
      var st = slots(this);
      if (st.started || st.dead) return;
      st.started = true;
      var port = this;
      st.queue.splice(0).forEach(function (init) {
        deliver(g, port, init, function () { return !st.dead; });
      });
    },
    close: function close() {
      var st = slots(this);
      st.dead = true;
      if (st.partner) slots(st.partner).partner = null;
      st.partner = null;
    },
  });
  // Setting `onmessage` starts the port (HTML "message port post message steps").
  var on = Object.getOwnPropertyDescriptor(Port.prototype, 'onmessage');
  Object.defineProperty(Port.prototype, 'onmessage', {
    get: on.get,
    set: brand(function (v) {
      on.set.call(this, v);
      this.start();
    }, 'set onmessage'),
    enumerable: true,
    configurable: true,
  });
  var MessageChannel = g.__frot_iface('MessageChannel', ['port1', 'port2'], function (inst) {
    var a = makePort(), b = makePort();
    entangle(a, b);
    Object.assign(slots(inst), { port1: a, port2: b });
  });

  // --- BroadcastChannel -------------------------------------------------------
  // Every open channel of the agent, in creation order (one origin per run),
  // as a record any window's code can read — a channel's slots are its own
  // realm's.
  var channels = shared.channels || (shared.channels = []);
  var Broadcast = g.__frot_iface('BroadcastChannel', null, function (inst, args) {
    if (!args.length)
      throw new TypeError('BroadcastChannel constructor: At least 1 argument required, but only 0 passed');
    var rec = { channel: inst, name: String(args[0]), closed: false, win: g };
    Object.assign(slots(inst), { name: rec.name, rec: rec });
    channels.push(rec);
  });
  g.__frot_extends(Broadcast, g.EventTarget);
  [MessageEvent, Broadcast].forEach(function (C) {
    Object.defineProperty(C, 'length', { value: 1, configurable: true });
  });
  g.__frot_ifaceops(Broadcast.prototype, {
    postMessage: function postMessage(message) {
      var own = slots(this).rec;
      if (own.closed)
        throw domError('InvalidStateError', 'An attempt was made to use an object that is not, or is no longer, usable');
      channels.slice().forEach(function (rec) {
        if (rec === own || rec.name !== own.name) return;
        deliver(rec.win, rec.channel, { data: clone(message, new Map()), origin: origin() }, function () {
          return !rec.closed;
        });
      });
    },
    close: function close() {
      var rec = slots(this).rec;
      rec.closed = true;
      var i = channels.indexOf(rec);
      if (i >= 0) channels.splice(i, 1);
    },
  });
  g.__frot_ifaceattrs(Broadcast.prototype, ['name']);
  g.__frot_onevent(Broadcast.prototype, 'onmessage');
  g.__frot_onevent(Broadcast.prototype, 'onmessageerror');

  // --- window.postMessage(message, targetOrigin | options, transfer) ---------
  function postMessage(message) {
    if (!arguments.length)
      throw new TypeError('Window.postMessage: 0 is not a valid argument count for any overload.');
    var target = this == null ? g : this, a = arguments[1];
    var o = a !== null && typeof a === 'object' ? a : { targetOrigin: a, transfer: arguments[2] };
    var to = o.targetOrigin === undefined ? '/' : String(o.targetOrigin);
    if (to !== '*' && to !== '/') {
      var parsed;
      try {
        parsed = new g.URL(to);
      } catch (e) {
        throw domError('SyntaxError', 'An invalid or illegal string was specified');
      }
      if (parsed.origin !== origin()) to = null;
    }
    var data = clone(message, new Map()), ports = transfer(o.transfer, null);
    if (to === null) return;
    var source = shared.entry || g.top;
    deliver(target, target, { data: data, origin: origin(), source: source, ports: ports });
  }
  Object.defineProperty(g, 'postMessage', {
    value: brand(postMessage, 'postMessage'),
    writable: true,
    enumerable: true,
    configurable: true,
  });
})(globalThis);
