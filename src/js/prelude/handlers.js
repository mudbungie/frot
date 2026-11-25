// Event handlers (bl-93c8; HTML "event handler attributes"): the `on*` IDL
// attributes and content attributes, measured on Firefox 153.0esr
// (src/js/tests/handlers/firefox153.json). A handler is ONE listener in
// events.js's registry, placed where it was first set and given up when set to
// null; its value is a callback, or an attribute's source text compiled on
// first use into `function on<type>(event) {\n<text>\n}` with the element, its
// form owner and its document on the scope chain. Running it is the "event
// handler processing algorithm": `false` returned cancels the event — `true`
// for the window's `onerror`, which is handed (message, source, lineno, colno,
// error) for a reported exception. A body or frameset forwards the
// window-reflecting handlers to its window. A parsed attribute was set before
// any script could add a listener, so it is found lazily — on first read of
// the IDL attribute or first dispatch of its type — and takes the FRONT of
// that type's listeners. Only events frot fires ever run one: frot dispatches
// no user input (js.md §11). Runs after events.js, before every module whose
// interfaces carry handlers.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;
  var reg = g.__frot_registry;
  var Fn = Function;

  // GlobalEventHandlers in Firefox's member order (identity.md §3.21); the
  // WindowEventHandlers a body/frameset forwards; the window-reflecting
  // GlobalEventHandlers a body/frameset forwards too.
  var GLOBAL = ('abort blur focus cancel auxclick beforeinput beforematch beforetoggle canplay '
    + 'canplaythrough change click close contentvisibilityautostatechange contextlost contextmenu '
    + 'command contextrestored copy cuechange cut dblclick drag dragend dragenter dragexit dragleave '
    + 'dragover dragstart drop durationchange emptied ended formdata input invalid keydown keypress '
    + 'keyup load loadeddata loadedmetadata loadstart mousedown mouseenter mouseleave mousemove '
    + 'mouseout mouseover mouseup wheel paste pause play playing progress ratechange reset resize '
    + 'scroll scrollend securitypolicyviolation seeked seeking select slotchange stalled submit '
    + 'suspend timeupdate volumechange waiting selectstart selectionchange toggle pointercancel '
    + 'pointerdown pointerup pointermove pointerout pointerover pointerenter pointerleave '
    + 'pointerrawupdate gotpointercapture lostpointercapture mozfullscreenchange mozfullscreenerror '
    + 'animationcancel animationend animationiteration animationstart transitioncancel transitionend '
    + 'transitionrun transitionstart webkitanimationend webkitanimationiteration '
    + 'webkitanimationstart webkittransitionend error').split(' ');
  var WINDOW = ('afterprint beforeprint beforeunload hashchange languagechange message messageerror '
    + 'offline online pagehide pageshow popstate rejectionhandled storage unhandledrejection unload '
    + 'gamepadconnected gamepaddisconnected').split(' ');
  var REFLECTED = { blur: 1, error: 1, focus: 1, load: 1, resize: 1, scroll: 1, scrollend: 1 };
  WINDOW.forEach(function (t) {
    REFLECTED[t] = 1;
  });
  var KNOWN = {};
  GLOBAL.concat(WINDOW).forEach(function (t) {
    KNOWN[t] = 1;
  });

  // target -> type -> {value, entry, owner}: `value` a callback, an object,
  // a string (uncompiled source) or null; `entry` the registry listener while
  // active; `owner` the element whose attribute the source came from.
  var states = new WeakMap();
  function find(target, type) {
    var t = states.get(target);
    return t && t[type];
  }
  function make(target, type) {
    var t = states.get(target);
    if (!t) states.set(target, (t = Object.create(null)));
    return t[type] || (t[type] = { value: null, entry: null, owner: null });
  }

  function tagOf(el) {
    var id = slots(el).id;
    return typeof id === 'number' ? g.__frot_tag(id) : null;
  }
  // Where an element's `on<type>` lives: the window for a body/frameset's
  // window-reflecting handlers, the element itself for the rest, nowhere for
  // a name no interface carries.
  function home(el, type) {
    var tag = tagOf(el);
    if ((tag === 'body' || tag === 'frameset') && REFLECTED[type]) return g;
    return KNOWN[type] && WINDOW.indexOf(type) < 0 ? el : null;
  }
  // The body element whose attributes stand for the window's handlers.
  function bodyOf() {
    return g.document && g.document.body;
  }

  // A parsed attribute's handler, found on first touch and put first.
  function discover(target, type) {
    if (find(target, type)) return;
    var st = make(target, type);
    var el = target === g ? (REFLECTED[type] ? bodyOf() : null) : target;
    if (!el || !tagOf(el) || (target !== g && home(el, type) !== target)) return;
    var src = g.__frot_attr(slots(el).id, 'on' + type);
    if (src === null || src === undefined) return;
    st.value = String(src);
    st.owner = el;
    activate(target, type, st, true);
  }
  function activate(target, type, st, front) {
    if (st.entry) return;
    st.entry = reg.add(target, type, function (ev) {
      run(target, type, st, ev);
    }, front);
  }
  function deactivate(target, type, st) {
    if (st.entry) reg.remove(target, type, st.entry);
    st.entry = null;
    st.value = null;
  }

  // The form owner of a listed element (form.js's `form`, bl-09c8).
  var LISTED = { button: 1, fieldset: 1, input: 1, object: 1, output: 1, select: 1, textarea: 1 };
  function formOf(el) {
    return LISTED[tagOf(el)] ? el.form : null;
  }
  // "Get the current value of the event handler": compile an attribute's
  // source on first use; a syntax error is reported and leaves null.
  function value(target, type, st) {
    if (typeof st.value !== 'string') return st.value;
    var params = target === g && type === 'error' ? 'event, source, lineno, colno, error' : 'event';
    try {
      st.value = Fn('__frot_d', '__frot_f', '__frot_e',
        'with (__frot_d) with (__frot_f) with (__frot_e) return function on' + type + '(' + params
        + ') {\n' + st.value + '\n};')(g.document, formOf(st.owner) || {}, st.owner);
    } catch (e) {
      st.value = null;
      g.__frot_report_exception(e);
    }
    return st.value;
  }
  // The event handler processing algorithm.
  function run(target, type, st, ev) {
    var cb = value(target, type, st);
    if (typeof cb !== 'function') return;
    var special = target === g && type === 'error' && ev instanceof g.ErrorEvent;
    var ret = special
      ? cb.call(target, ev.message, ev.filename, ev.lineno, ev.colno, ev.error)
      : cb.call(target, ev);
    if (special ? ret === true : ret === false) ev.preventDefault();
  }

  // The IDL attribute: enumerable, configurable, default null; any object is
  // kept (a non-callable one is never called), anything else is null.
  function accessor(proto, type, where) {
    var key = 'on' + type;
    Object.defineProperty(proto, key, {
      get: brand(function () {
        var t = where(this, type);
        discover(t, type);
        return value(t, type, make(t, type));
      }, 'get ' + key),
      set: brand(function (v) {
        var t = where(this, type);
        discover(t, type);
        var st = make(t, type);
        if (v === null || (typeof v !== 'object' && typeof v !== 'function')) return deactivate(t, type, st);
        st.value = v;
        st.owner = null;
        activate(t, type, st, false);
      }, 'set ' + key),
      enumerable: true,
      configurable: true,
    });
  }
  function self(o) {
    return o == null ? g : o;
  }
  function forwarded(el, type) {
    return home(el, type) || el;
  }
  function toWindow() {
    return g;
  }

  // The content attribute: setting one gives the handler its source text
  // (keeping a place it already has); removing one that is there gives the
  // place up — except a body/frameset's forwarded one, which Firefox leaves
  // on the window (measured). The handler's target is found, and a parsed
  // attribute's place claimed, BEFORE the write changes the attribute.
  function handlerOf(id, name) {
    name = String(name).toLowerCase();
    if (name.slice(0, 2) !== 'on') return null;
    var type = name.slice(2), el = g.__frot_wrap(id), target = home(el, type);
    if (!target) return null;
    discover(target, type);
    return { el: el, type: type, target: target, st: make(target, type), had: g.__frot_attr(id, name) != null };
  }
  var raw = { set: g.__frot_set_attr, remove: g.__frot_remove_attr };
  function seam(name, fn) {
    Object.defineProperty(g, name, { value: brand(fn, name), configurable: true, writable: true });
  }
  seam('__frot_set_attr', function (id, name, value) {
    var h = handlerOf(id, name);
    raw.set(id, name, value);
    if (!h) return;
    h.st.value = String(value);
    h.st.owner = h.el;
    activate(h.target, h.type, h.st, false);
  });
  seam('__frot_remove_attr', function (id, name) {
    var h = handlerOf(id, name);
    raw.remove(id, name);
    if (h && h.had && h.target !== g) deactivate(h.target, h.type, h.st);
  });

  // Where the accessors are: GlobalEventHandlers on HTMLElement, SVGElement,
  // Document and the window (its own properties, as on the binary), the
  // window's own after them, and the forwarding WindowEventHandlers on
  // body/frameset. The event-bearing interfaces of other modules make theirs
  // through `__frot_onevent`, which is this same maker.
  GLOBAL.forEach(function (t) {
    [g.HTMLElement.prototype, g.SVGElement.prototype].forEach(function (p) {
      accessor(p, t, forwarded);
    });
    accessor(g.Document.prototype, t, self);
  });
  ['devicemotion', 'deviceorientation', 'deviceorientationabsolute'].concat(GLOBAL, WINDOW).forEach(function (t) {
    accessor(g, t, toWindow);
  });
  WINDOW.forEach(function (t) {
    [g.HTMLBodyElement.prototype, g.HTMLFrameSetElement.prototype].forEach(function (p) {
      accessor(p, t, toWindow);
    });
  });
  var onEvent = function (proto, key) {
    accessor(proto, key.slice(2), self);
  };
  [['__frot_onevent', onEvent], ['__frot_handler_discover', discover]].forEach(function (p) {
    Object.defineProperty(g, p[0], { value: brand(p[1], p[0]), configurable: true, writable: true });
  });
  Object.defineProperty(g, '__frot_handler_names', {
    value: GLOBAL.map(function (t) {
      return 'on' + t;
    }).join(' '),
    configurable: true,
  });
})(globalThis);
