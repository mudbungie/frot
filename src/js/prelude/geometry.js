// Geometry facade (js.md §8) — getBoundingClientRect / offset* / getComputedStyle
// layered over the two geometry syscalls. Both route into the host's
// per-generation Styles+Layout cache; wrappers cache nothing. Augments the
// Element prototypes from dom.js, so this module loads after it.
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
  var ops = g.__frot_ifaceops;
  var brand = g.__frot_brand;

  // [x, y, w, h] in the fixed 1280px viewport; the all-zero box for a box-less
  // node (display:none / non-rendered / non-element). Structural estimates, not
  // pixel truth (layout.md §6).
  function box(id) {
    return g.__frot_rect(id);
  }

  var P = g.Element.prototype;
  var H = g.HTMLElement.prototype;

  // --- DOMRect ---------------------------------------------------------------
  // Measured: `DOMRectReadOnly.prototype` carries `toJSON`, then x, y, width,
  // height, top, right, bottom, left as getter-only accessors; `DOMRect`
  // inherits from it and re-declares only the four settable ones. Both are
  // constructable (`bl-e65f`): four unrestricted doubles, each defaulting to 0,
  // and a static `fromRect(dict)` on each. A rect's state is those four in its
  // slot and the interface it was made as; the edges are derived, never stored
  // (two representations of one fact drift), with Gecko's NaN-safe min/max — a
  // negative size flips the edges, any NaN makes the edge NaN, and `-0` survives.
  var CORNERS = ['x', 'y', 'width', 'height'];
  function min(a, b) {
    return a !== a || b !== b ? NaN : b < a ? b : a;
  }
  function max(a, b) {
    return a !== a || b !== b ? NaN : a < b ? b : a;
  }
  var EDGES = {
    top: function (r) {
      return min(r[1], r[1] + r[3]);
    },
    right: function (r) {
      return max(r[0], r[0] + r[2]);
    },
    bottom: function (r) {
      return max(r[1], r[1] + r[3]);
    },
    left: function (r) {
      return min(r[0], r[0] + r[2]);
    },
  };
  // The rect `obj` holds, or Gecko's receiver error. A DOMRect setter needs a
  // rect made as a DOMRect; every other member takes either.
  function held(obj, member, name) {
    var st = slots(obj);
    if (st.rect && (name !== 'DOMRect' || st.iface === name)) return st.rect;
    throw new TypeError("'" + member + "' called on an object that does not implement interface " +
      name + '.');
  }
  // WebIDL `unrestricted double`: ToNumber, `undefined` the member's default 0.
  function dbl(v) {
    return v === undefined ? 0 : +v;
  }
  function shape(inst, name, x, y, w, h) {
    slots(inst).rect = [dbl(x), dbl(y), dbl(w), dbl(h)];
    slots(inst).iface = name;
    return inst;
  }
  function rectIface(name) {
    var C = iface(name, {}, function (inst, args) {
      shape(inst, name, args[0], args[1], args[2], args[3]);
    });
    // DOMRectInit's members are read in lexicographic order (measured).
    // The dictionary is optional, so `fromRect.length` is 0 (measured).
    C.fromRect = brand(function fromRect() {
      var init = arguments[0];
      var d = init === undefined || init === null ? {} : init;
      if (typeof d !== 'object' && typeof d !== 'function') {
        throw new TypeError(name + ".fromRect: Argument 1 can't be converted to a dictionary.");
      }
      var h = d.height;
      var w = d.width;
      var x = d.x;
      return shape(Object.create(C.prototype), name, x, d.y, w, h);
    }, 'fromRect');
    return C;
  }
  var DOMRectReadOnly = rectIface('DOMRectReadOnly');
  ops(DOMRectReadOnly.prototype, {
    toJSON: function toJSON() {
      held(this, 'toJSON', 'DOMRectReadOnly');
      var self = this;
      var o = {};
      CORNERS.concat(Object.keys(EDGES)).forEach(function (k) {
        o[k] = self[k];
      });
      return o;
    },
  });
  var readers = {};
  CORNERS.forEach(function (k, i) {
    readers[k] = function () {
      return held(this, 'get ' + k, 'DOMRectReadOnly')[i];
    };
  });
  Object.keys(EDGES).forEach(function (k) {
    readers[k] = function () {
      return EDGES[k](held(this, 'get ' + k, 'DOMRectReadOnly'));
    };
  });
  attrs(DOMRectReadOnly.prototype, readers);

  var DOMRect = rectIface('DOMRect');
  g.__frot_extends(DOMRect, DOMRectReadOnly);
  CORNERS.forEach(function (k, i) {
    Object.defineProperty(DOMRect.prototype, k, {
      get: brand(function () {
        return held(this, 'get ' + k, 'DOMRect')[i];
      }, 'get ' + k),
      set: brand(function (v) {
        held(this, 'set ' + k, 'DOMRect')[i] = +v;
      }, 'set ' + k),
      enumerable: true,
      configurable: true,
    });
  });

  P.getBoundingClientRect = function () {
    var r = box(slots(this).id);
    return shape(Object.create(DOMRect.prototype), 'DOMRect', r[0], r[1], r[2], r[3]);
  };

  function offset(index) {
    return function () {
      return box(slots(this).id)[index];
    };
  }
  attrs(H, { offsetTop: offset(1), offsetLeft: offset(0), offsetWidth: offset(2), offsetHeight: offset(3) });

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
