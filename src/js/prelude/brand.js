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
  function toString() {
    var name = reg.get(this);
    if (name !== undefined) {
      return 'function ' + name + '() {\n    [native code]\n}';
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
      reg.set(fn, name === undefined ? fn.name : name);
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
      throw new TypeError('Illegal constructor');
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

  // Expose all three as NON-enumerable globals: the persona/capability modules
  // reach them, but a page walking `Object.keys(window)`/`for..in` never sees
  // them (the raw `__frot_*` syscalls' enumerability is a separate, documented
  // residual — js.md §7). Branded native so their own toString does not leak.
  [
    ['__frot_brand', brand],
    ['__frot_iface', iface],
    ['__frot_domerror', domError],
  ].forEach(function (pair) {
    brand(pair[1], pair[0]);
    Object.defineProperty(g, pair[0], {
      value: pair[1],
      configurable: true,
      writable: true,
    });
  });
})(globalThis);
