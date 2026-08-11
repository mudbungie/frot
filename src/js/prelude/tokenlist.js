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

  // Pinned Firefox 140esr supported-token tables (see header).
  var LINK_REL = [
    'modulepreload', 'manifest', 'preload', 'prefetch', 'dns-prefetch',
    'stylesheet', 'next', 'alternate', 'preconnect', 'icon', 'search',
  ];
  var ANCHOR_REL = ['noreferrer', 'noopener', 'opener'];

  function tokens(list) {
    var v = list._el.getAttribute(list._attr);
    return v ? v.trim().split(/\s+/) : [];
  }
  function put(list, toks) {
    list._el.setAttribute(list._attr, toks.join(' '));
  }

  var DOMTokenList = g.__frot_iface('DOMTokenList', {
    length: function () {
      return tokens(this).length;
    },
  });
  // `value` reflects the whole attribute and, unlike the __frot_iface accessors,
  // is writable — Firefox gives it a setter.
  Object.defineProperty(DOMTokenList.prototype, 'value', {
    get: g.__frot_brand(function () {
      return this._el.getAttribute(this._attr) || '';
    }, 'get value'),
    set: g.__frot_brand(function (v) {
      this._el.setAttribute(this._attr, String(v));
    }, 'set value'),
    enumerable: true,
    configurable: true,
  });
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
    toggle: function toggle(t, force) {
      var has = tokens(this).indexOf(String(t)) >= 0;
      var on = force === undefined ? !has : !!force;
      if (on) this.add(t);
      else this.remove(t);
      return on;
    },
    replace: function replace(oldT, newT) {
      var list = tokens(this);
      var i = list.indexOf(String(oldT));
      if (i < 0) return false;
      list[i] = String(newT);
      put(this, list);
      return true;
    },
    supports: function supports(t) {
      if (!this._sup)
        throw new TypeError(
          "Operation is not supported: DOMTokenList doesn't have supported tokens defined."
        );
      return this._sup.indexOf(String(t).toLowerCase()) >= 0;
    },
    toString: function toString() {
      return this.value;
    },
    forEach: A.forEach,
    entries: A.entries,
    keys: A.keys,
    values: A.values,
  };
  Object.keys(methods).forEach(function (k) {
    Object.defineProperty(DOMTokenList.prototype, k, {
      value: g.__frot_brand(methods[k], k),
      writable: true,
      enumerable: true,
      configurable: true,
    });
  });
  Object.defineProperty(DOMTokenList.prototype, Symbol.iterator, {
    value: A.values,
    writable: true,
    configurable: true,
  });

  // Live indexed access (`classList[0]`) without caching: a Proxy resolves
  // integer indices against a fresh token split, everything else against the
  // prototype — the same live-over-the-attribute rule as every other read.
  var handler = {
    get: function (t, p, r) {
      if (typeof p === 'string' && /^\d+$/.test(p)) {
        var toks = tokens(t);
        return +p < toks.length ? toks[+p] : undefined;
      }
      return Reflect.get(t, p, r);
    },
    has: function (t, p) {
      if (typeof p === 'string' && /^\d+$/.test(p)) return +p < tokens(t).length;
      return Reflect.has(t, p);
    },
  };
  function makeList(el, attr, supported) {
    var t = Object.create(DOMTokenList.prototype, {
      _el: { value: el },
      _attr: { value: attr },
      _sup: { value: supported || null },
    });
    return new Proxy(t, handler);
  }

  // classList on every element; the `class` attribute defines no supported
  // tokens, so its supports() throws (per spec, as Firefox does).
  Object.defineProperty(proto, 'classList', {
    configurable: true,
    get: function () {
      return makeList(this, 'class', null);
    },
  });
  // relList only where Firefox has it — link, and a/area/form (which share one
  // Gecko table); elsewhere it stays undefined so feature detection is honest.
  Object.defineProperty(proto, 'relList', {
    configurable: true,
    get: function () {
      var t = this.tagName;
      if (t === 'LINK') return makeList(this, 'rel', LINK_REL);
      if (t === 'A' || t === 'AREA' || t === 'FORM') return makeList(this, 'rel', ANCHOR_REL);
      return undefined;
    },
  });
})(globalThis);
