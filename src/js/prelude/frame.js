// Iframe windows (js.md §14). A connected <iframe> has a window: a second realm
// with its own intrinsics and its own about:blank document, opened by the
// __frot_open_frame syscall the first time a script asks for it and kept in the
// element's slots, so `f.contentWindow === f.contentWindow`. Opening on first
// read rather than at insertion is unobservable except by cost — a page full of
// iframes nobody scripts pays for none of them. A frame never navigates: frot
// fetches no framed document, so a `src` frame stays on its initial
// about:blank, which is what a browser shows until that navigation completes.
// Every window opened here schedules into this window's loop (`__frot_loop`,
// loop.js): one agent, one event loop. Removing or moving the iframe closes
// its window, and `window[i]` indexes the frames (bl-2aaa).
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
  var opened = [];
  function windowOf(el) {
    if (!el.isConnected) return null;
    var s = slots(el);
    if (!s.window) {
      var w = open(shared);
      w.__frot_link(g, g.top, el);
      s.window = w;
      opened.push(el);
    }
    return s.window;
  }
  // Removing an iframe closes its window (bl-2aaa, measured): `closed` true,
  // `parent`/`top`/`frameElement` null, `length` 0, its document and `eval`
  // still usable; inserted again — or moved, which is a removal and an
  // insertion — it gets a new window.
  function close(el) {
    var w = slots(el).window;
    slots(el).window = null;
    opened.splice(opened.indexOf(el), 1);
    w.__frot_close();
  }
  // This window's links, as the binary shapes them: `parent` replaceable,
  // `top` unforgeable, `frameElement` read-only; a top window is its own
  // parent and top. Its opener links a frame; closing unlinks it.
  var link = { parent: g, top: g, frameElement: null, closed: false };
  function accessor(key, set, configurable) {
    var d = {
      get: brand(function () {
        return link[key];
      }, 'get ' + key),
      enumerable: true,
      configurable: configurable,
    };
    if (set)
      d.set = brand(function (v) {
        Object.defineProperty(g, key, { value: v, writable: true, enumerable: true, configurable: true });
      }, 'set ' + key);
    Object.defineProperty(g, key, d);
  }
  accessor('closed', false, true);
  accessor('parent', true, true);
  accessor('top', false, false);
  accessor('frameElement', false, true);
  [['__frot_link', function (parent, top, el) {
    link.parent = parent;
    link.top = top;
    link.frameElement = el;
  }], ['__frot_close', function () {
    link = { parent: null, top: null, frameElement: null, closed: true };
    Object.defineProperty(g, 'length', { value: 0, writable: true, enumerable: true, configurable: true });
  }]].forEach(function (p) {
    Object.defineProperty(g, p[0], { value: p[1], configurable: true });
  });

  // `window[i]` is the i-th iframe's window (measured: enumerable, and gone
  // past `length`). The indices are accessors over the live list, opening a
  // window when read like `contentWindow`; they are re-laid whenever a
  // mutation moves an iframe, which is also when a frame may close.
  var indexed = 0;
  function frameList() {
    return g.document.getElementsByTagName('iframe');
  }
  function index(i) {
    Object.defineProperty(g, i, {
      get: brand(function () {
        var el = frameList()[i];
        return el ? windowOf(el) : undefined;
      }, 'get ' + i),
      enumerable: true,
      configurable: true,
    });
  }
  function relay() {
    opened.slice().forEach(function (el) {
      if (!el.isConnected) close(el);
    });
    var n = frameList().length;
    for (var i = 0; i < n; i++) index(i);
    for (; indexed > n; indexed--) delete g[indexed - 1];
    indexed = n;
  }
  // Whether a mutation of the subtree at `id` can touch a frame: it is, or
  // holds, an iframe.
  function framed(id) {
    return g.__frot_tag(id) === 'iframe' || g.__frot_query(id, 'iframe').length > 0;
  }
  var raw = { insert: g.__frot_insert_child, detach: g.__frot_detach };
  function seam(name, fn) {
    Object.defineProperty(g, name, { value: brand(fn, name), configurable: true, writable: true });
  }
  seam('__frot_insert_child', function (parent, child, before) {
    var moving = framed(child) && opened.filter(function (el) {
      return el.isConnected && (slots(el).id === child || g.__frot_wrap(child).contains(el));
    });
    var why = raw.insert(parent, child, before);
    if (moving) {
      moving.forEach(close);
      relay();
    }
    return why;
  });
  seam('__frot_detach', function (id) {
    var r = raw.detach(id);
    if (framed(id)) relay();
    return r;
  });
  relay();
  // Member order on the interface is nativebrand.js's TAGGED list.
  // `name` is the frame's target name: the window's named property (window.js).
  g.__frot_tagdef('IFrame', 'name', g.__frot_reflect('name'));
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
