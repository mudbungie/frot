// Geometry facade (js.md §8) — getBoundingClientRect / offset* / getComputedStyle
// layered over the two geometry syscalls. Both route into the host's
// per-generation Styles+Layout cache; wrappers cache nothing. Augments the
// Node prototype from dom.js, so this module loads after it.
//
// The two objects it hands back are real interfaces (bl-643d): a rect was a
// plain object literal owning eight numbers, and a computed style owned its one
// method, where Gecko's own both own NOTHING and answer off a prototype. Every
// shape below — member lists, Gecko's member order, which members have setters,
// and the three error messages — is read off Firefox 153.0esr (identity.md
// §3.17), never derived from spec text, which orders members differently.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var iface = g.__frot_iface;
  var attrs = g.__frot_ifaceattrs;
  var rw = g.__frot_rwattrs;
  var ops = g.__frot_ifaceops;
  var brand = g.__frot_brand;

  // [x, y, w, h] in the fixed 1280px viewport; the all-zero box for a box-less
  // node (display:none / non-rendered / non-element). Structural estimates, not
  // pixel truth (layout.md §6).
  function box(id) {
    return g.__frot_rect(id);
  }

  var P = g.Node.prototype;

  // --- DOMRect ---------------------------------------------------------------
  // Measured: `DOMRectReadOnly.prototype` carries `toJSON`, then x, y, width,
  // height, top, right, bottom, left as getter-only accessors; `DOMRect`
  // inherits from it and re-declares only the four settable ones. y grows
  // downward, so top/left are the origin and right/bottom the far edges —
  // derived from the four, never stored beside them (two representations of one
  // fact drift).
  var CORNERS = ['x', 'y', 'width', 'height'];
  var EDGES = {
    top: function (s) {
      return s.y;
    },
    right: function (s) {
      return s.x + s.width;
    },
    bottom: function (s) {
      return s.y + s.height;
    },
    left: function (s) {
      return s.x;
    },
  };
  var DOMRectReadOnly = iface('DOMRectReadOnly');
  ops(DOMRectReadOnly.prototype, {
    toJSON: function toJSON() {
      var self = this;
      var o = {};
      CORNERS.concat(Object.keys(EDGES)).forEach(function (k) {
        o[k] = self[k];
      });
      return o;
    },
  });
  attrs(DOMRectReadOnly.prototype, CORNERS);
  var edgeAttrs = {};
  Object.keys(EDGES).forEach(function (k) {
    edgeAttrs[k] = function () {
      return EDGES[k](slots(this));
    };
  });
  attrs(DOMRectReadOnly.prototype, edgeAttrs);

  var DOMRect = iface('DOMRect');
  Object.setPrototypeOf(DOMRect.prototype, DOMRectReadOnly.prototype);
  rw(DOMRect.prototype, CORNERS);

  P.getBoundingClientRect = function () {
    var r = box(slots(this).id);
    var rect = Object.create(DOMRect.prototype);
    var st = slots(rect);
    CORNERS.forEach(function (k, i) {
      st[k] = r[i];
    });
    return rect;
  };

  function offset(index) {
    return function () {
      return box(slots(this).id)[index];
    };
  }
  Object.defineProperty(P, 'offsetLeft', { get: offset(0) });
  Object.defineProperty(P, 'offsetTop', { get: offset(1) });
  Object.defineProperty(P, 'offsetWidth', { get: offset(2) });
  Object.defineProperty(P, 'offsetHeight', { get: offset(3) });

  // --- getComputedStyle ------------------------------------------------------
  // Two interfaces, as on the binary: `CSSStyleDeclaration` holds the generic
  // API (item, getPropertyValue, getPropertyPriority, setProperty,
  // removeProperty, then cssText, length, parentRule) and `CSSStyleProperties`
  // inherits it and holds the per-property reflections — which is why the tag
  // reads `[object CSSStyleProperties]`. A computed style is READ-ONLY, and its
  // three mutating members throw `NoModificationAllowedError` with the messages
  // measured verbatim below rather than silently accepting a write.
  //
  // NOT asserted, recorded instead (identity.md §3.17): a real computed style
  // enumerates all 383 longhands — `length` is 383, `item(0)` is `accent-color`,
  // and each index is an own property. frot computes the js.md §8 subset, so it
  // can neither list 383 honestly nor pass its four off as that list. `length`
  // is 0 and the indices are absent; `item()` answers "" for every index, which
  // is what a real one answers past its end.
  var PROPS = { display: 'display', visibility: 'visibility', order: 'order', flexDirection: 'flex-direction' };

  // The three refusals, message-exact. `%` takes the property name for the two
  // that name it; the cssText setter's message names none, so its phrase has no
  // placeholder and the substitution is a no-op.
  function refuse(member, phrase) {
    return function (arg) {
      throw g.__frot_domerror(
        'NoModificationAllowedError',
        'CSSStyleDeclaration.' + member + ": Can't " + phrase.replace('%', String(arg))
      );
    };
  }

  var CSSStyleDeclaration = iface('CSSStyleDeclaration');
  function computed(inst, prop) {
    return g.__frot_computed_style(slots(inst).id, String(prop));
  }
  ops(CSSStyleDeclaration.prototype, {
    item: function item() {
      return '';
    },
    getPropertyValue: function getPropertyValue(prop) {
      return computed(this, prop);
    },
    getPropertyPriority: function getPropertyPriority() {
      return '';
    },
    setProperty: refuse('setProperty', "set value for property '%' in computed style"),
    removeProperty: refuse('removeProperty', "remove property '%' from computed style"),
  });
  Object.defineProperty(CSSStyleDeclaration.prototype, 'cssText', {
    get: brand(function () {
      return '';
    }, 'get cssText'),
    set: brand(refuse('cssText setter', 'set cssText on computed style'), 'set cssText'),
    enumerable: true,
    configurable: true,
  });
  attrs(CSSStyleDeclaration.prototype, {
    length: function () {
      return 0;
    },
    parentRule: function () {
      return null;
    },
  });

  var CSSStyleProperties = iface('CSSStyleProperties');
  Object.setPrototypeOf(CSSStyleProperties.prototype, CSSStyleDeclaration.prototype);
  var reflections = {};
  Object.keys(PROPS).forEach(function (name) {
    reflections[name] = function () {
      return computed(this, PROPS[name]);
    };
  });
  attrs(CSSStyleProperties.prototype, reflections);

  g.getComputedStyle = function (el) {
    var s = Object.create(CSSStyleProperties.prototype);
    slots(s).id = slots(el).id;
    return s;
  };
})(globalThis);
