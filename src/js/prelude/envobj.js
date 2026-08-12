// The environment's four ambient objects as real INTERFACES (js.md §7,
// identity.md §3.17, bl-643d): `localStorage`/`sessionStorage`, `location`,
// `history`, and what `matchMedia()` returns. Split out of env.js on that seam
// (which keeps the global aliasing, the cookie bridge and the viewport facts).
//
// They were plain object literals, so every member was an own property of the
// instance and a page walking one read frot's implementation instead of a WebIDL
// interface. All four shapes below are READ off Firefox 153.0esr over Marionette
// (identity.md §3.17) — the member lists, Gecko's member ORDER, and the
// descriptors — never derived from spec text, which orders members differently.
//
// Loads after events.js (MediaQueryList inherits its EventTarget) and after
// brand.js (the registry); before any page script.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var iface = g.__frot_iface;
  var attrs = g.__frot_ifaceattrs;
  var ops = g.__frot_ifaceops;
  var slots = g.__frot_slots;

  // --- Storage --------------------------------------------------------------
  // Measured prototype: key, getItem, setItem, removeItem, clear, length,
  // constructor — operations first, `length` the one attribute. localStorage and
  // sessionStorage share ONE Storage.prototype on the binary, so they share one
  // here. The instances own nothing; the map lives in the closure, which is
  // stronger than the slots WeakMap and needs no key.
  var Storage = iface('Storage');
  ops(Storage.prototype, {
    key: function key(i) {
      var ks = Object.keys(slots(this).map);
      return i >= 0 && i < ks.length ? ks[i] : null;
    },
    getItem: function getItem(k) {
      var map = slots(this).map;
      k = String(k);
      return k in map ? map[k] : null;
    },
    setItem: function setItem(k, v) {
      slots(this).map[String(k)] = String(v);
    },
    removeItem: function removeItem(k) {
      delete slots(this).map[String(k)];
    },
    clear: function clear() {
      slots(this).map = Object.create(null);
    },
  });
  attrs(Storage.prototype, {
    length: function () {
      return Object.keys(slots(this).map).length;
    },
  });

  function makeStorage() {
    // A Proxy gives the bracket/dot sugar (`store.foo`, `store[k] = v`) a real
    // Storage's named-property getter exposes; the named lookup falls through to
    // the interface first, so the members above keep answering off the prototype.
    //
    // The proxy, not its target, is the instance every caller holds, so the
    // proxy is what keys the slot store — otherwise a method invoked as
    // `store.setItem(…)` runs with `this` = the proxy and reads an empty record.
    // `Reflect.get` forwards the same receiver to the `length` accessor, so
    // exactly one object has backing state and `clear()` is visible either way.
    var store;
    store = new Proxy(Object.create(Storage.prototype), {
      get: function (t, p) {
        if (p in t) return Reflect.get(t, p, store);
        var map = slots(store).map;
        return typeof p === 'string' && p in map ? map[p] : undefined;
      },
      set: function (t, p, v) {
        if (p in t) return false;
        slots(store).map[String(p)] = String(v);
        return true;
      },
      has: function (t, p) {
        return p in t || (typeof p === 'string' && p in slots(store).map);
      },
      deleteProperty: function (t, p) {
        delete slots(store).map[String(p)];
        return true;
      },
      // A stored key is an OWN, enumerable, writable, configurable property of a
      // real Storage — measured: after two setItem calls
      // `Object.getOwnPropertyNames(localStorage)` lists both, and `Object.keys`
      // agrees. Without these two traps the target owns nothing and every key
      // frot stored was invisible to a page that enumerated the object.
      // Insertion order is frot's, not Gecko's: the binary answered in neither
      // insertion nor sorted order (its internal hash order), which is not a
      // fact a deterministic tool can reproduce (VISION principle 1), so this
      // asserts the membership it measured and not an order it cannot.
      ownKeys: function () {
        return Object.keys(slots(store).map);
      },
      getOwnPropertyDescriptor: function (t, p) {
        var map = slots(store).map;
        if (typeof p === 'string' && p in map) {
          return { value: map[p], writable: true, enumerable: true, configurable: true };
        }
        return Object.getOwnPropertyDescriptor(t, p);
      },
    });
    slots(store).map = Object.create(null);
    return store;
  }
  g.localStorage = makeStorage();
  g.sessionStorage = makeStorage();

  // --- Location -------------------------------------------------------------
  // The measurement contradicted the premise this ball was filed under, so read
  // it before changing it: `Location.prototype` owns ONLY `constructor`, and a
  // real `location` owns its entire surface as own properties — the one platform
  // object in this file that does. (Gecko puts them on the instance because
  // Location is `[LegacyUnforgeable]`: every member is non-configurable so a
  // cross-origin page cannot redefine it.) frot's shape was therefore already
  // right; what was wrong was the ORDER, two missing members, and descriptors a
  // page can overwrite where Gecko's cannot be.
  //
  // Measured order: href, origin, protocol, host, hostname, port, pathname,
  // search, hash, ancestorOrigins, assign, replace, reload, toString, valueOf —
  // attributes first here, then the operations, the reverse of every prototype.
  // `href` and the URL parts have setters; `origin`/`ancestorOrigins` do not;
  // `valueOf` is the one non-enumerable member.
  var Location = iface('Location');
  var DOMStringList = iface('DOMStringList');
  ops(DOMStringList.prototype, {
    item: function item(i) {
      return i >= 0 && i < slots(this).list.length ? slots(this).list[i] : null;
    },
    contains: function contains(s) {
      return slots(this).list.indexOf(String(s)) >= 0;
    },
  });
  attrs(DOMStringList.prototype, {
    length: function () {
      return slots(this).list.length;
    },
  });
  // A top-level document has no ancestors, so the list is empty — the honest
  // value, not a stand-in: frot has one arena and no frames (js.md §11).
  var origins = Object.create(DOMStringList.prototype);
  slots(origins).list = [];

  var L = g.__frot_location();
  var loc = Object.create(Location.prototype);
  // Navigation is the denial (js.md §7/§11): assigning `href` or calling
  // assign/replace/reload is observable, so each is counted, never silent.
  function navigate() {
    g.__frot_denied();
  }
  ['href', 'origin', 'protocol', 'host', 'hostname', 'port', 'pathname', 'search', 'hash'].forEach(
    function (k) {
      Object.defineProperty(loc, k, {
        get: brand(function () {
          return L[k];
        }, 'get ' + k),
        set: k === 'origin' ? undefined : brand(navigate, 'set ' + k),
        enumerable: true,
        configurable: false,
      });
    }
  );
  Object.defineProperty(loc, 'ancestorOrigins', {
    get: brand(function () {
      return origins;
    }, 'get ancestorOrigins'),
    enumerable: true,
    configurable: false,
  });
  [
    ['assign', navigate], ['replace', navigate], ['reload', navigate],
    ['toString', function toString() {
      return L.href;
    }],
    ['valueOf', function valueOf() {
      return this;
    }],
  ].forEach(function (pair) {
    Object.defineProperty(loc, pair[0], {
      value: brand(pair[1], pair[0]),
      writable: false,
      enumerable: pair[0] !== 'valueOf',
      configurable: false,
    });
  });
  // `location = url` / `window.location = url` are navigation too (js.md §11),
  // and `document.location` is the SAME object on the binary, not a copy.
  [g, g.document].forEach(function (o) {
    Object.defineProperty(o, 'location', {
      configurable: true,
      get: brand(function () {
        return loc;
      }, 'get location'),
      set: brand(navigate, 'set location'),
    });
  });

  // --- History --------------------------------------------------------------
  // Measured prototype: go, back, forward, pushState, replaceState, length,
  // scrollRestoration, state, constructor. Routers read `history.state` on first
  // render; absent, they throw. pushState/replaceState set `.state` (the one fact
  // read back); pushState also bumps `length`. They do NOT mutate `location`,
  // which stays the honest fetched URL (§7) — frot takes ONE impression, and SPA
  // routers pick their initial route from that location. go/back/forward are
  // no-ops: there is nowhere to go. Born fresh, discarded at exit (§7).
  var History = iface('History');
  ops(History.prototype, {
    go: function go() {},
    back: function back() {},
    forward: function forward() {},
    pushState: function pushState(state) {
      var st = slots(this);
      st.state = state;
      st.length += 1;
    },
    replaceState: function replaceState(state) {
      slots(this).state = state;
    },
  });
  // Measured order, and measured mutability: `scrollRestoration` is the one
  // settable attribute of the three, so it is declared between the two
  // read-only ones rather than after them.
  attrs(History.prototype, ['length']);
  g.__frot_rwattrs(History.prototype, ['scrollRestoration']);
  attrs(History.prototype, ['state']);
  var history = Object.create(History.prototype);
  var hst = slots(history);
  hst.length = 1;
  hst.scrollRestoration = 'auto';
  hst.state = null;
  g.history = history;

  // --- MediaQueryList -------------------------------------------------------
  // Measured prototype: addListener, removeListener, media, matches, onchange,
  // constructor — and its parent is EventTarget, so addEventListener/
  // removeEventListener/dispatchEvent come from there rather than being three
  // more own members (the shape bl-6438 established for every event target).
  //
  // One authority for media-query semantics (src/css/media.rs): @media blocks in
  // the CSS cascade and matchMedia here both evaluate through the
  // __frot_media_matches syscall, so CSS and JS can never disagree. The viewport
  // is fixed, so `matches` cannot change and the listeners never fire — the
  // declared silence, not a wrong answer.
  var MediaQueryList = iface('MediaQueryList');
  Object.setPrototypeOf(MediaQueryList.prototype, g.EventTarget.prototype);
  ops(MediaQueryList.prototype, {
    addListener: function addListener() {},
    removeListener: function removeListener() {},
  });
  attrs(MediaQueryList.prototype, ['media', 'matches']);
  g.__frot_onevent(MediaQueryList.prototype, 'onchange');

  g.matchMedia = brand(function matchMedia(query) {
    query = String(query);
    var mql = Object.create(MediaQueryList.prototype);
    var st = slots(mql);
    st.media = query;
    st.matches = g.__frot_media_matches(query);
    return mql;
  }, 'matchMedia');
})(globalThis);
