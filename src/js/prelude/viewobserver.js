// IntersectionObserver / ResizeObserver (js.md §7, bl-07ab) — genuine INITIAL
// delivery, computed from the same per-generation §8 geometry that serves
// getBoundingClientRect (structural estimates, layout.md §6). These do NOT
// ride MutationObserver's coattails, and presence-but-never-firing was argued
// and REJECTED: a real browser always delivers an initial batch per observed
// target (IO's first update pass, RO's initial size report), so a silent
// observer contradicts every real Firefox on the API's first use — and a
// lazy-load library that would fall back to eager loading under ABSENCE
// instead waits forever, silently. The initial batch here is a truthful
// observation: frot's viewport sits at scroll offset 0 and never scrolls,
// resizes, or animates (js.md §11), exactly a real browser left untouched after
// load. The declared residual (identity.md §11): later DOM-mutation-driven
// geometry changes produce no further entries. Delivery is a task on the one
// §5 timer queue (browsers deliver observer batches from the rendering steps —
// a task, not a microtask); no second scheduler exists.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var attrs = g.__frot_ifaceattrs;
  // Observer bookkeeping and every entry's fields live in brand.js's one
  // instance-state WeakMap, behind prototype accessors, so an observer and an
  // entry each own nothing — the shape a real Gecko one has (bl-3bdc, §3.16).
  var slots = g.__frot_slots;

  // A constructable, Firefox-shaped interface (the worker.js pattern; brand.js's
  // __frot_iface is for the Illegal-constructor entry/size classes below).
  function ctor(name, build) {
    var holder = {};
    holder[name] = function (cb, opts) {
      if (!(this instanceof holder[name])) {
        throw new TypeError("Constructor " + name + " requires 'new'");
      }
      if (typeof cb !== 'function') {
        throw new TypeError(name + ' constructor: Argument 1 is not callable.');
      }
      build(this, cb, opts);
    };
    var C = brand(holder[name], name);
    Object.defineProperty(C.prototype, Symbol.toStringTag, { value: name, configurable: true });
    Object.defineProperty(g, name, { value: C, configurable: true, writable: true });
    return C;
  }

  function box(id) {
    var r = g.__frot_rect(id);
    return { left: r[0], top: r[1], w: r[2], h: r[3] };
  }
  function domRect(left, top, w, h) {
    return { x: left, y: top, width: w, height: h, top: top, left: left, right: left + w, bottom: top + h };
  }

  // One delivery task per observation wave: observe() marks the target fresh
  // and schedules; the task computes entries from the CURRENT generation's
  // geometry (post any same-script mutations) and invokes the callback once.
  // An emptied wave (everything unobserved before the task ran) invokes
  // nothing — a real observer never calls back with zero entries.
  function schedule(obs, entry) {
    var st = slots(obs);
    if (st.due) return;
    st.due = true;
    g.setTimeout(function () {
      st.due = false;
      var fresh = st.fresh;
      st.fresh = [];
      if (!fresh.length) return;
      var entries = [];
      for (var i = 0; i < fresh.length; i++) entries.push(entry(fresh[i]));
      st.cb.call(obs, entries, obs);
    }, 0);
  }
  function watch(obs, target, iface) {
    if (!target || typeof slots(target).id !== 'number') {
      throw new TypeError(iface + '.observe: Argument 1 does not implement interface Element.');
    }
    var st = slots(obs);
    for (var i = 0; i < st.targets.length; i++) {
      if (slots(st.targets[i]).id === slots(target).id) return;
    }
    st.targets.push(target);
    st.fresh.push(target);
  }
  function unwatch(obs, target) {
    var id = target && slots(target).id;
    function keep(t) {
      return slots(t).id !== id;
    }
    var st = slots(obs);
    st.targets = st.targets.filter(keep);
    st.fresh = st.fresh.filter(keep);
  }
  function baseSlots(inst, cb) {
    var st = slots(inst);
    st.cb = cb;
    st.targets = [];
    st.fresh = [];
    st.due = false;
  }
  function clear(obs) {
    var st = slots(obs);
    st.targets = [];
    st.fresh = [];
  }

  // --- IntersectionObserver -------------------------------------------------
  var IOEntry = g.__frot_iface('IntersectionObserverEntry', {});
  // rootMargin: 1–4 px/% components, CSS-shorthand expanded; Gecko throws on
  // anything else, and so does this.
  function margins(str) {
    var parts = String(str === undefined || str === '' ? '0px' : str).trim().split(/\s+/);
    if (parts.length > 4) throw new SyntaxError('rootMargin must have 1 to 4 components');
    var out = [];
    for (var i = 0; i < parts.length; i++) {
      var m = /^(-?\d+(?:\.\d+)?)(px|%)$/.exec(parts[i]);
      if (!m) throw new SyntaxError('rootMargin must be specified in pixels or percent');
      out.push({ v: parseFloat(m[1]), pct: m[2] === '%' });
    }
    if (out.length === 1) out = [out[0], out[0], out[0], out[0]];
    else if (out.length === 2) out = [out[0], out[1], out[0], out[1]];
    else if (out.length === 3) out = [out[0], out[1], out[2], out[1]];
    return out;
  }
  function thresholds(t) {
    var arr = t === undefined ? [0] : Array.isArray(t) ? t.slice() : [t];
    if (!arr.length) arr = [0];
    for (var i = 0; i < arr.length; i++) {
      arr[i] = Number(arr[i]);
      if (!(arr[i] >= 0 && arr[i] <= 1)) {
        throw new RangeError('Threshold values must be numbers between 0 and 1');
      }
    }
    arr.sort(function (a, b) {
      return a - b;
    });
    return Object.freeze(arr);
  }
  var IO = ctor('IntersectionObserver', function (inst, cb, opts) {
    opts = opts || {};
    baseSlots(inst, cb);
    var st = slots(inst);
    st.root = opts.root === undefined ? null : opts.root;
    st.margin = margins(opts.rootMargin);
    st.thresholds = thresholds(opts.threshold);
  });
  attrs(IO.prototype, {
    root: null,
    rootMargin: function () {
      return slots(this).margin
        .map(function (m) {
          return m.v + (m.pct ? '%' : 'px');
        })
        .join(' ');
    },
    thresholds: null,
  });
  function ioEntry(inst, target) {
    var t = box(slots(target).id);
    var root = slots(inst).root;
    var rb = root && typeof slots(root).id === 'number'
      ? box(slots(root).id)
      : { left: 0, top: 0, w: g.innerWidth, h: g.innerHeight };
    var m = slots(inst).margin; // [top, right, bottom, left]; % of the root dimension
    var mt = m[0].pct ? (m[0].v * rb.h) / 100 : m[0].v;
    var mr = m[1].pct ? (m[1].v * rb.w) / 100 : m[1].v;
    var mb = m[2].pct ? (m[2].v * rb.h) / 100 : m[2].v;
    var ml = m[3].pct ? (m[3].v * rb.w) / 100 : m[3].v;
    rb = { left: rb.left - ml, top: rb.top - mt, w: rb.w + ml + mr, h: rb.h + mt + mb };
    // The §8 all-zero box is the "no box" sentinel (display:none/non-rendered):
    // an unrendered target honestly does not intersect.
    var noBox = t.w === 0 && t.h === 0 && t.left === 0 && t.top === 0;
    var ix = Math.max(t.left, rb.left);
    var iy = Math.max(t.top, rb.top);
    var ir = Math.min(t.left + t.w, rb.left + rb.w);
    var ib = Math.min(t.top + t.h, rb.top + rb.h);
    var hit = !noBox && ir >= ix && ib >= iy;
    var iw = hit ? ir - ix : 0;
    var ih = hit ? ib - iy : 0;
    var area = t.w * t.h;
    var e = Object.create(IOEntry.prototype);
    var st = slots(e);
    st.time = g.performance.now();
    st.target = target;
    st.boundingClientRect = domRect(t.left, t.top, t.w, t.h);
    st.rootBounds = domRect(rb.left, rb.top, rb.w, rb.h);
    st.intersectionRect = domRect(hit ? ix : 0, hit ? iy : 0, iw, ih);
    st.intersectionRatio = hit ? (area > 0 ? Math.min((iw * ih) / area, 1) : 1) : 0;
    st.isIntersecting = hit;
    return e;
  }
  attrs(IOEntry.prototype, ['time', 'target', 'boundingClientRect', 'rootBounds',
    'intersectionRect', 'intersectionRatio', 'isIntersecting']);
  IO.prototype.observe = brand(function observe(target) {
    var self = this;
    watch(this, target, 'IntersectionObserver');
    schedule(this, function (t) {
      return ioEntry(self, t);
    });
  }, 'observe');
  IO.prototype.unobserve = brand(function unobserve(target) {
    unwatch(this, target);
  }, 'unobserve');
  IO.prototype.disconnect = brand(function disconnect() {
    clear(this);
  }, 'disconnect');
  IO.prototype.takeRecords = brand(function takeRecords() {
    var self = this;
    var st = slots(this);
    var fresh = st.fresh;
    st.fresh = [];
    return fresh.map(function (t) {
      return ioEntry(self, t);
    });
  }, 'takeRecords');

  // --- ResizeObserver -------------------------------------------------------
  var ROEntry = g.__frot_iface('ResizeObserverEntry', {});
  var ROSize = g.__frot_iface('ResizeObserverSize', {});
  attrs(ROSize.prototype, ['inlineSize', 'blockSize']);
  function roSize(w, h) {
    var s = Object.create(ROSize.prototype);
    slots(s).inlineSize = w;
    slots(s).blockSize = h;
    return s;
  }
  function roEntry(target) {
    var t = box(slots(target).id);
    var e = Object.create(ROEntry.prototype);
    var st = slots(e);
    st.target = target;
    // contentRect's origin is the box's own padding edge — 0,0 in frot's
    // borderless model (§8), where content box == border box; the device-pixel
    // box matches at the profile's devicePixelRatio of 1. Coherent, not a
    // shortcut: those equalities are the model's own contract.
    st.contentRect = domRect(0, 0, t.w, t.h);
    st.borderBoxSize = [roSize(t.w, t.h)];
    st.contentBoxSize = [roSize(t.w, t.h)];
    st.devicePixelContentBoxSize = [roSize(t.w, t.h)];
    return e;
  }
  attrs(ROEntry.prototype, ['target', 'contentRect', 'borderBoxSize',
    'contentBoxSize', 'devicePixelContentBoxSize']);
  var RO = ctor('ResizeObserver', function (inst, cb) {
    baseSlots(inst, cb);
  });
  RO.prototype.observe = brand(function observe(target) {
    watch(this, target, 'ResizeObserver');
    schedule(this, roEntry);
  }, 'observe');
  RO.prototype.unobserve = brand(function unobserve(target) {
    unwatch(this, target);
  }, 'unobserve');
  RO.prototype.disconnect = brand(function disconnect() {
    clear(this);
  }, 'disconnect');
})(globalThis);
