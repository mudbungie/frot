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
})(globalThis);
