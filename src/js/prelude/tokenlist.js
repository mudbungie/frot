// DOMTokenList (js.md §3, bl-3a36) — the spec-named interface for every token
// list the DOM exposes, extending the bl-e5c3 collection invariant: one maker,
// live over the backing attribute (the arena stays the one source of truth — a
// list caches nothing). `classList` and `relList` are both instances; the field
// trial's Vite preload polyfill runs `link.relList.supports('modulepreload')`
// at module top level and, absent relList, fell through to an undefined
// MutationObserver and died. The interface is WebIDL-shaped the way Gecko
// exposes it: `new DOMTokenList()` throws Illegal constructor, `length`/`value`
// are prototype accessors, indexed access is live (a Proxy over the token
// split), and the iteration methods ARE the `Array.prototype` ones.
//
// `supports()` answers from the pinned Firefox 140.12.0esr supported-token
// tables (dom/html/HTMLLinkElement.cpp `SUPPORTED_REL_VALUES_BASE` with the
// default-on `manifest`/`modulepreload` prefs; dom/base/Element.cpp
// `sAnchorAndFormRelValues`), ASCII case-insensitively; an attribute with no
// supported-token definition (`class`) throws the coherent TypeError. Loads
// after elem2.js (extends the Node prototype dom.js built).
(function (g) {
  'use strict';
  var proto = g.Node.prototype;
  var A = Array.prototype;
  // The backing element/attribute/supported-token set live in brand.js's one
  // instance-state WeakMap: a real Gecko `classList` owns no properties, and the
  // `_el`/`_attr`/`_sup` slots this replaces were readable straight out of
  // `Object.getOwnPropertyNames` (bl-3bdc, identity.md §3.16).
  var slots = g.__frot_slots;

  // Pinned Firefox 140esr supported-token tables (see header).
  var LINK_REL = [
    'modulepreload', 'manifest', 'preload', 'prefetch', 'dns-prefetch',
    'stylesheet', 'next', 'alternate', 'preconnect', 'icon', 'search',
  ];
  var ANCHOR_REL = ['noreferrer', 'noopener', 'opener'];

  function tokens(list) {
    var st = slots(list);
    var v = st.el.getAttribute(st.attr);
    return v ? v.trim().split(/\s+/) : [];
  }
  function put(list, toks) {
    var st = slots(list);
    st.el.setAttribute(st.attr, toks.join(' '));
  }

  // The methods are defined first and `length`/`value` after, because that is the
  // own-property order Gecko's `DOMTokenList.prototype` enumerates in — measured
  // (identity.md §3.15): operations, then attributes, then `constructor`.
  var DOMTokenList = g.__frot_iface('DOMTokenList');
  var methods = {
    item: function item(i) {
      var t = tokens(this);
      return i >>> 0 < t.length ? t[i >>> 0] : null;
    },
    contains: function contains(t) {
      return tokens(this).indexOf(String(t)) >= 0;
    },
    add: function add() {
      var list = tokens(this);
      for (var i = 0; i < arguments.length; i++)
        if (list.indexOf(String(arguments[i])) < 0) list.push(String(arguments[i]));
      put(this, list);
    },
    remove: function remove() {
      var list = tokens(this);
      for (var i = 0; i < arguments.length; i++) {
        var j = list.indexOf(String(arguments[i]));
        if (j >= 0) list.splice(j, 1);
      }
      put(this, list);
    },
    replace: function replace(oldT, newT) {
      var list = tokens(this);
      var i = list.indexOf(String(oldT));
      if (i < 0) return false;
      list[i] = String(newT);
      put(this, list);
      return true;
    },
    toggle: function toggle(t, force) {
      var has = tokens(this).indexOf(String(t)) >= 0;
      var on = force === undefined ? !has : !!force;
      if (on) this.add(t);
      else this.remove(t);
      return on;
    },
    supports: function supports(t) {
      if (!slots(this).sup)
        throw new TypeError(
          "Operation is not supported: DOMTokenList doesn't have supported tokens defined."
        );
      return slots(this).sup.indexOf(String(t).toLowerCase()) >= 0;
    },
    keys: A.keys,
    values: A.values,
    entries: A.entries,
    forEach: A.forEach,
    toString: function toString() {
      return this.value;
    },
  };
  Object.keys(methods).forEach(function (k) {
    Object.defineProperty(DOMTokenList.prototype, k, {
      value: g.__frot_brand(methods[k], k),
      writable: true,
      enumerable: true,
      configurable: true,
    });
  });
  g.__frot_ifaceattrs(DOMTokenList.prototype, {
    length: function () {
      return tokens(this).length;
    },
  });
  Object.defineProperty(DOMTokenList.prototype, 'value', {
    get: g.__frot_brand(function () {
      var st = slots(this);
      return st.el.getAttribute(st.attr) || '';
    }, 'get value'),
    set: g.__frot_brand(function (v) {
      var st = slots(this);
      st.el.setAttribute(st.attr, String(v));
    }, 'set value'),
    enumerable: true,
    configurable: true,
  });
  Object.defineProperty(DOMTokenList.prototype, Symbol.iterator, {
    value: A.values,
    writable: true,
    configurable: true,
  });

  // Live indexed access (`classList[0]`) without caching: a Proxy resolves
  // integer indices against a fresh token split, everything else against the
  // prototype — the same live-over-the-attribute rule as every other read. The
  // traps close over the PROXY, not the target, because the proxy is what a page
  // holds and therefore what `this` is inside every prototype method — one
  // object, so one state entry (the target owns nothing, and neither does it).
  function makeList(el, attr, supported) {
    var list = new Proxy(Object.create(DOMTokenList.prototype), {
      get: function (t, p, r) {
        if (typeof p === 'string' && /^\d+$/.test(p)) {
          var toks = tokens(list);
          return +p < toks.length ? toks[+p] : undefined;
        }
        return Reflect.get(t, p, r);
      },
      has: function (t, p) {
        if (typeof p === 'string' && /^\d+$/.test(p)) return +p < tokens(list).length;
        return Reflect.has(t, p);
      },
    });
    var st = slots(list);
    st.el = el;
    st.attr = attr;
    st.sup = supported || null;
    return list;
  }

  // Both lists are WebIDL `[PutForwards=value]`, so both descriptors come from
  // elem.js's one maker (bl-273b): `el.classList = 'a b'` runs
  // `el.classList.value = 'a b'` and lands on the class attribute, exactly as
  // Chrome 139 does — the list object itself is never replaced.
  //
  // classList on every element; the `class` attribute defines no supported
  // tokens, so its supports() throws (per spec, as Firefox does).
  Object.defineProperty(
    proto,
    'classList',
    g.__frot_forwards(function () {
      return makeList(this, 'class', null);
    }, 'value')
  );
  // relList only where Firefox has it — link, and a/area/form (which share one
  // Gecko table); elsewhere it stays undefined so feature detection is honest.
  Object.defineProperty(
    proto,
    'relList',
    g.__frot_forwards(function () {
      var t = this.tagName;
      if (t === 'LINK') return makeList(this, 'rel', LINK_REL);
      if (t === 'A' || t === 'AREA' || t === 'FORM') return makeList(this, 'rel', ANCHOR_REL);
      return undefined;
    }, 'value')
  );
})(globalThis);
