// Geometry facade (js.md §8) — getBoundingClientRect / offset* / getComputedStyle
// layered over the two geometry syscalls. Both route into the host's
// per-generation Styles+Layout cache; wrappers cache nothing. Augments the
// Node prototype from dom.js, so this module loads after it.
(function (g) {
  'use strict';

  // [x, y, w, h] in the fixed 1280px viewport; the all-zero box for a box-less
  // node (display:none / non-rendered / non-element). Structural estimates, not
  // pixel truth (layout.md §6).
  function box(id) {
    return g.__frot_rect(id);
  }

  var P = g.Node.prototype;

  // A DOMRect-shaped plain object; y grows downward, so top/left are the origin
  // and right/bottom are the far edges.
  P.getBoundingClientRect = function () {
    var r = box(this._id);
    var x = r[0], y = r[1], w = r[2], h = r[3];
    return { x: x, y: y, width: w, height: h, top: y, left: x, right: x + w, bottom: y + h };
  };

  function offset(index) {
    return function () {
      return box(this._id)[index];
    };
  }
  Object.defineProperty(P, 'offsetLeft', { get: offset(0) });
  Object.defineProperty(P, 'offsetTop', { get: offset(1) });
  Object.defineProperty(P, 'offsetWidth', { get: offset(2) });
  Object.defineProperty(P, 'offsetHeight', { get: offset(3) });

  // getComputedStyle: only the js.md §8 subset the cascade computes; every other
  // property reads back "". camelCase accessors map to their CSS property name.
  var PROPS = { display: 'display', visibility: 'visibility', order: 'order', flexDirection: 'flex-direction' };

  function ComputedStyle(id) {
    this._id = id;
  }
  ComputedStyle.prototype.getPropertyValue = function (prop) {
    return g.__frot_computed_style(this._id, String(prop));
  };
  Object.keys(PROPS).forEach(function (name) {
    Object.defineProperty(ComputedStyle.prototype, name, {
      get: function () {
        return g.__frot_computed_style(this._id, PROPS[name]);
      },
    });
  });

  g.getComputedStyle = function (el) {
    return new ComputedStyle(el._id);
  };
})(globalThis);
