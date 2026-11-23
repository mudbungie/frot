// Iframe windows (js.md §14). A connected <iframe> has a window: a second realm
// with its own intrinsics and its own about:blank document, opened by the
// __frot_open_frame syscall the first time a script asks for it and kept in the
// element's slots, so `f.contentWindow === f.contentWindow`. Opening on first
// read rather than at insertion is unobservable except by cost — a page full of
// iframes nobody scripts pays for none of them. A frame never navigates: frot
// fetches no framed document, so a `src` frame stays on its initial
// about:blank, which is what a browser shows until that navigation completes.
// Every window opened here schedules into this window's loop (`__frot_loop`,
// loop.js): one agent, one event loop.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;
  var shared = g.__frot_loop;
  var open = g.__frot_open_frame;
  delete g.__frot_loop;
  delete g.__frot_open_frame;

  // A disconnected iframe has no window (measured: null before insertion and
  // after removal). The links a frame's window carries to its parent are set
  // once, here, by the parent that owns it.
  function windowOf(el) {
    if (!el.isConnected) return null;
    var s = slots(el);
    if (!s.window) {
      var w = open(shared);
      w.parent = g;
      w.top = g.top;
      w.frameElement = el;
      s.window = w;
    }
    return s.window;
  }
  // Member order on the interface is nativebrand.js's TAGGED list.
  g.__frot_tagdef('IFrame', 'contentDocument', {
    get: brand(function () {
      var w = windowOf(this);
      return w ? w.document : null;
    }, 'get contentDocument'),
  });
  g.__frot_tagdef('IFrame', 'contentWindow', {
    get: brand(function () {
      return windowOf(this);
    }, 'get contentWindow'),
  });

  // `frames` is the window itself; `length` counts its child frames — every
  // iframe in the document, opened or not, so it is a query and never a tally.
  // Both are [Replaceable]: a page's own `var length` must win, not throw.
  g.frames = g;
  Object.defineProperty(g, 'length', {
    get: brand(function () {
      return g.document.getElementsByTagName('iframe').length;
    }, 'get length'),
    set: brand(function (v) {
      Object.defineProperty(g, 'length', { value: v, writable: true, enumerable: true, configurable: true });
    }, 'set length'),
    enumerable: true,
    configurable: true,
  });
})(globalThis);
