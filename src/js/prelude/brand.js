// Native-code branding + web-interface shaping (identity.md §8/§10, bl-3972 /
// bl-3926). ONE Function.prototype.toString wrapper backed by ONE registry is
// the single mechanism by which frot's JS-implemented web APIs read as native
// (`function name() { [native code] }`) instead of disclosing their prelude
// source or the syscall name `frot`. Every persona surface here — and the six
// later capability balls (canvas/WebGL/audio/indexedDB/Worker/permissions) —
// registers through the same non-enumerable `__frot_brand`/`__frot_iface`
// helpers; nothing hand-patches Function.prototype.toString a second time.
// Runs FIRST so later prelude modules can brand as they define.
(function (g) {
  'use strict';

  // fn -> display name. A WeakMap so a branded function pins no extra reference
  // and never becomes enumerable state on any object a page can walk.
  var reg = new WeakMap();
  var realToString = Function.prototype.toString;

  // The one wrapper. A registered function renders as native code under its
  // display name; everything else (page/library code) still sees true source.
  //
  // An accessor's display name is its SPEC name — `get width` / `set onmessage`
  // — because that is what `fn.name` must read. Its `toString()`, though, drops
  // the prefix: measured on Firefox 153.0esr (`bl-1ab7`, identity.md §3.12),
  //   Object.getOwnPropertyDescriptor(Screen.prototype,'width').get
  //     .name     === 'get width'
  //     .toString() === 'function width() {\n    [native code]\n}'
  // and the setter renders under the bare name too. Rendering `function get
  // width()` — as this did until the surfaces were read off a real binary — is a
  // string no Firefox emits, so every branded getter was a tell.
  function toString() {
    var name = reg.get(this);
    if (name !== undefined) {
      return 'function ' + name.replace(/^(get|set) /, '') + '() {\n    [native code]\n}';
    }
    return realToString.call(this);
  }
  // The wrapper and the original both read native, so `toString.toString()` and
  // `Function.prototype.toString.toString()` do not betray the shim.
  reg.set(toString, 'toString');
  reg.set(realToString, 'toString');
  Function.prototype.toString = toString;

  // Mark `fn` native under `name` (default: its own name). Idempotent, total —
  // a non-function is ignored so callers need no guard. The registry, not the
  // call site, owns the disguise (identity.md §8: "do not hand-patch each call").
  function brand(fn, name) {
    if (typeof fn === 'function') {
      var display = name === undefined ? fn.name : name;
      reg.set(fn, display);
      // `fn.name` is the other half of the disguise and the half a page reads
      // directly: a real native accessor reports `get width` (measured, §3.12)
      // where an unnamed prelude function expression reports `''`. Set it here,
      // at the one site that already owns the display name, so no call site has
      // to remember. `name` is configurable-but-not-writable on functions.
      try {
        Object.defineProperty(fn, 'name', { value: display, configurable: true });
      } catch (e) { /* frozen or exotic: the toString disguise still holds */ }
    }
    return fn;
  }

  // Build a branded DOM/Web interface constructor `name` with accessor
  // properties from `accessors` (a plain object of getter functions). The result
  // is Firefox-shaped: `new Ctor()` throws "Illegal constructor"; the prototype
  // carries @@toStringTag (so `Object.prototype.toString.call(inst)` is
  // `[object <name>]`); each property is an enumerable, configurable accessor on
  // the prototype with `set === undefined` and a native-looking getter; and the
  // constructor is published as a NON-enumerable global (so `x instanceof Name`
  // and `window.Name` work, exactly as a browser exposes its interfaces).
  // Returns the constructor; the caller makes its singleton via
  // `Object.create(Ctor.prototype)`.
  function iface(name, accessors) {
    var holder = {};
    holder[name] = function () {
      // Measured on 153.0esr (`bl-1ab7`): the message ends in a period.
      throw new TypeError('Illegal constructor.');
    };
    var Ctor = holder[name];
    brand(Ctor, name);
    var proto = Ctor.prototype;
    Object.defineProperty(proto, Symbol.toStringTag, {
      value: name,
      configurable: true,
    });
    Object.keys(accessors).forEach(function (key) {
      var get = brand(accessors[key], 'get ' + key);
      Object.defineProperty(proto, key, {
        get: get,
        enumerable: true,
        configurable: true,
      });
    });
    Object.defineProperty(g, name, { value: Ctor, configurable: true, writable: true });
    return Ctor;
  }

  // A DOMException-shaped error: quickjs has no DOMException, so a plain Error
  // carrying the spec `name` is the closest tell. One maker, because the name is
  // the whole fact and five modules need it — crypto/idb/abort's rejections and
  // the DOM's own hierarchy errors (bl-273b) — and five copies of a four-line
  // constructor is five chances to drift.
  function domError(name, message) {
    var e = new Error(message);
    e.name = name;
    return e;
  }

  // An event-handler IDL accessor (`onchange`/`onclick`/…): enumerable, native,
  // defaults null, settable, and never firing (the silence is each capability's
  // declared residual). One maker for the same reason `domError` is one — four
  // modules shape these (permissions, worker, screen, idb) and four copies of an
  // accessor pair is four chances for one of them to drift out of Firefox's shape.
  function onEvent(proto, key) {
    var slot = '_on_' + key;
    Object.defineProperty(proto, key, {
      get: brand(function () {
        return Object.prototype.hasOwnProperty.call(this, slot) ? this[slot] : null;
      }, 'get ' + key),
      set: brand(function (fn) {
        Object.defineProperty(this, slot, {
          value: typeof fn === 'function' ? fn : null,
          configurable: true,
          writable: true,
        });
      }, 'set ' + key),
      enumerable: true,
      configurable: true,
    });
  }

  // Expose all four as NON-enumerable globals: the persona/capability modules
  // reach them, but a page walking `Object.keys(window)`/`for..in` never sees
  // them (the raw `__frot_*` syscalls' enumerability is a separate, documented
  // residual — js.md §7). Branded native so their own toString does not leak.
  [
    ['__frot_brand', brand],
    ['__frot_iface', iface],
    ['__frot_domerror', domError],
    ['__frot_onevent', onEvent],
  ].forEach(function (pair) {
    brand(pair[1], pair[0]);
    Object.defineProperty(g, pair[0], {
      value: pair[1],
      configurable: true,
      writable: true,
    });
  });
})(globalThis);
