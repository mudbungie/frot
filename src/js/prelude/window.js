// The Window interface (bl-226d), as Firefox 153.0esr has it
// (src/js/tests/window/): the window's prototype is `Window.prototype` (its
// own members: `constructor` and @@toStringTag), then the named properties
// object (`[object WindowProperties]`, no interface object of its own), then
// `EventTarget.prototype` — so the window inherits addEventListener like every
// EventTarget. The named properties object is inert here: a Proxy in the
// global's prototype chain makes quickjs answer `undefined` for every
// unresolvable identifier instead of throwing ReferenceError (its global
// lookup takes a proxy's `get` and returns), so window named access waits on
// an engine-level exotic object (bl-de60).
(function (g) {
  'use strict';
  var Window = g.__frot_iface('Window');
  Object.setPrototypeOf(Window, g.EventTarget);
  var named = Object.create(g.EventTarget.prototype);
  Object.defineProperty(named, Symbol.toStringTag, { value: 'WindowProperties', configurable: true });
  // Its prototype is immutable and it takes no properties (Firefox throws on
  // both).
  Object.preventExtensions(named);
  Object.setPrototypeOf(Window.prototype, named);
  // The window is an EventTarget by inheritance now: its own copies of the
  // trio (events.js) give way to EventTarget.prototype's. The engine's own
  // `globalThis[@@toStringTag]` ('global') goes too: the tag is Window's.
  ['addEventListener', 'removeEventListener', 'dispatchEvent', Symbol.toStringTag].forEach(function (k) {
    delete g[k];
  });
  Object.setPrototypeOf(g, Window.prototype);

  // The window's everyday members (bl-d8f7), own properties as Firefox
  // 153.0esr has them. frot is a person-less, unscrolled, single-window
  // browser, so each answers as that browser would: a dialog is dismissed at
  // once (alert undefined, confirm false, prompt null), a popup is blocked
  // (open null), scrolling stays at the top, focus and printing do nothing.
  var brand = g.__frot_brand, st = g.__frot_slots(g);
  st.name = st.status = '';
  function own(k, desc) {
    desc.enumerable = desc.configurable = true;
    if (desc.get) desc.get = brand(desc.get, 'get ' + k);
    if (desc.set) desc.set = brand(desc.set, 'set ' + k);
    Object.defineProperty(g, k, desc);
  }
  ['name', 'status'].forEach(function (k) {
    own(k, { get: function () { return st[k]; }, set: function (v) { st[k] = String(v); } });
  });
  // [Replaceable]: assigning one replaces the accessor with the value.
  function replaceable(k, get) {
    own(k, { get: get, set: function (v) {
      Object.defineProperty(g, k, { value: v, writable: true, enumerable: true, configurable: true });
    } });
  }
  replaceable('origin', function () { return g.location.origin; });
  replaceable('opener', function () { return null; });
  replaceable('event', g.__frot_current_event);
  var answers = { scroll: undefined, scrollTo: undefined, scrollBy: undefined, focus: undefined,
    blur: undefined, stop: undefined, print: undefined, alert: undefined, confirm: false,
    prompt: null, open: null, close: undefined };
  Object.keys(answers).forEach(function (k) {
    g[k] = brand(function () { return answers[k]; }, k);
  });

  // Selection: frot's is always empty — nothing selects, and Range is not
  // built — so every member answers for no range.
  var Selection = g.__frot_iface('Selection');
  g.__frot_ifaceops(Selection.prototype, {
    getRangeAt: function getRangeAt(i) {
      throw g.__frot_domerror('IndexSizeError', 'Selection.getRangeAt: ' + (i >>> 0) + ' is out of range');
    },
    removeAllRanges: function removeAllRanges() {},
    empty: function empty() {},
    containsNode: function containsNode() { return false; },
    toString: function toString() { return ''; },
  });
  var NONE = { anchorNode: null, anchorOffset: 0, focusNode: null, focusOffset: 0, isCollapsed: true,
    rangeCount: 0, type: 'None', direction: 'none' };
  var attrs = {};
  Object.keys(NONE).forEach(function (k) {
    attrs[k] = function () { return NONE[k]; };
  });
  g.__frot_ifaceattrs(Selection.prototype, attrs);
  var selection = Object.create(Selection.prototype);
  g.getSelection = brand(function getSelection() { return selection; }, 'getSelection');
  g.__frot_ifaceops(g.Document.prototype, {
    getSelection: function getSelection() { return selection; },
  });
})(globalThis);
