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
  // Every interface the registry builds, so `sealInterfaces` can put
  // `constructor` last on all of them AFTER the later modules have finished
  // adding methods and event handlers to their prototypes. Doing it inside
  // `iface` alone is not enough — anything defined after the call lands past the
  // constructor — and asking each call site to re-seal is the call-site
  // discipline this registry exists to remove.
  var built = [];

  // --- Per-instance backing state --------------------------------------------
  // ONE WeakMap for the whole prelude, for the same reason `reg` above is one:
  // state kept HERE is state no page can walk. Measured on Firefox 153.0esr
  // (`bl-3bdc`, identity.md §3.16), `Object.getOwnPropertyNames` of a real
  // Notification / audio node / canvas context / observer is `[]` — every value a
  // Gecko instance answers comes from a prototype accessor and the instance owns
  // nothing. frot backed those accessors with `_`-prefixed own properties, which
  // `getOwnPropertyNames` reports whether or not they are enumerable (so hiding
  // them from `for..in` hid nothing), and a page walking one instance read frot's
  // implementation instead of a WebIDL interface. `slots(inst)` is the store they
  // moved into: off the instance, weakly keyed so it dies with it, invisible to
  // every own-property probe — names and symbols alike.
  //
  // Total, like `brand`: a primitive keys no WeakMap, so it gets a throwaway
  // entry rather than "invalid value used as weak map key". That keeps the
  // WebIDL argument checks reading as they read on a real browser —
  // `observer.observe(5)` must fail with Gecko's "does not implement interface
  // Element", not with an engine complaint about frot's store.
  var backing = new WeakMap();
  function slots(obj) {
    var s = backing.get(obj);
    if (s === undefined) {
      s = {};
      if (obj !== null && (typeof obj === 'object' || typeof obj === 'function')) {
        backing.set(obj, s);
      }
    }
    return s;
  }
  function slotGetter(key) {
    return function () {
      return slots(this)[key];
    };
  }
  // Read/write WebIDL attributes over those slots. `after(key, value)` runs on
  // assignment for the ones whose write has a consequence — a fold into a
  // fingerprint digest, a reflected attribute write; the plain settable case
  // passes none.
  function rwAttrs(proto, names, after) {
    names.forEach(function (key) {
      Object.defineProperty(proto, key, {
        get: brand(slotGetter(key), 'get ' + key),
        set: brand(function (v) {
          slots(this)[key] = v;
          if (after) after.call(this, key, v);
        }, 'set ' + key),
        enumerable: true,
        configurable: true,
      });
    });
    return proto;
  }

  // Define WebIDL attributes (enumerable native accessors) on a prototype. Split
  // out of `iface` so a module can interleave in Gecko's own member order —
  // operations first, then attributes (measured across 41 prototypes, §3.15) —
  // instead of being forced to declare every attribute before every method.
  // `accessors` may also be a plain ARRAY of names, or carry `null` for a name:
  // that attribute reads the instance's backing slot of the same name. It is by
  // far the common case — most WebIDL attributes ARE their stored value — so the
  // registry writes that getter and the call site states only the name.
  function attrs(proto, accessors) {
    var spec = accessors;
    if (Array.isArray(accessors)) {
      spec = {};
      accessors.forEach(function (k) {
        spec[k] = null;
      });
    }
    Object.keys(spec).forEach(function (key) {
      Object.defineProperty(proto, key, {
        get: brand(spec[key] || slotGetter(key), 'get ' + key),
        enumerable: true,
        configurable: true,
      });
    });
    return proto;
  }

  // Define WebIDL *operations* on a prototype, from a plain object of functions.
  // The companion to `attrs`, and the other half of Gecko's member order —
  // operations first, then attributes (§3.15). A WebIDL operation is a writable,
  // enumerable, configurable data property, which is exactly what an assignment
  // makes, so the only thing this adds over `proto.f = fn` is the branding every
  // call site had to remember: an unbranded prelude function discloses its source
  // through `toString()`, and its `.name` reads `''` where Gecko's reads the
  // member name. Eight modules were spelling that pair out per member.
  function ops(proto, table) {
    Object.keys(table).forEach(function (key) {
      proto[key] = brand(table[key], key);
    });
    return proto;
  }

  // `body(inst, args)` makes the interface CONSTRUCTABLE — it shapes each new
  // instance and may throw the one synchronous WebIDL check. Omit it and `new
  // Ctor()` throws "Illegal constructor.", which is right for the interfaces a
  // page can only receive (Storage, Location, Performance) and WRONG for the
  // ones it can build (Event, Headers, Response, XMLHttpRequest, Notification):
  // an interface that refuses `new` where Firefox allows it is as loud a tell as
  // one that allows it where Firefox refuses. Constructable interfaces used to
  // be a private helper in permissions.js; this is that helper, moved to the one
  // registry so they all seal `constructor` last and publish the same way.
  function iface(name, accessors, body) {
    var holder = {};
    holder[name] = body
      ? function () {
        body(this, arguments);
      }
      : function () {
        // Measured on 153.0esr (`bl-1ab7`): the message ends in a period.
        throw new TypeError('Illegal constructor.');
      };
    var Ctor = holder[name];
    brand(Ctor, name);
    var proto = Ctor.prototype;
    // `constructor` LAST. A JS function's `prototype` is born owning it FIRST,
    // but every one of the 41 Gecko interface prototypes read in `bl-706b`
    // (identity.md §3.15) lists it last — operations, then attributes, then
    // `constructor`. Deleting and redefining it moves it to the end of the
    // insertion order while keeping the descriptor WebIDL gives it
    // (writable, non-enumerable, configurable), so `Object.getOwnPropertyNames`
    // reads like a real interface instead of like a JS class.
    delete proto.constructor;
    // `accessors` is optional: dom.js builds several presence-only interfaces
    // (`new NodeList()` throws, and that is the whole contract).
    attrs(proto, accessors || {});
    // The tag is a symbol, so it sorts after every string key in own-property
    // order regardless of insertion and can never displace a named member.
    Object.defineProperty(proto, Symbol.toStringTag, {
      value: name,
      configurable: true,
    });
    Object.defineProperty(g, name, { value: Ctor, configurable: true, writable: true });
    built.push(Ctor);
    return Ctor;
  }

  // Move `constructor` to the end of every registered interface prototype, and
  // of any extra constructors handed in (the hand-rolled ones — EventTarget and
  // Worker's constructable twin — which do not come through `iface`). Called
  // once, from the final sweep, when every prototype is complete. Measured basis:
  // all 41 Gecko interface prototypes read in `bl-706b` list `constructor` last
  // (identity.md §3.15); a JS function's prototype is born owning it first.
  function sealInterfaces(extra) {
    built.concat(extra || []).forEach(function (Ctor) {
      var proto = Ctor && Ctor.prototype;
      if (!proto) return;
      delete proto.constructor;
      Object.defineProperty(proto, 'constructor', {
        value: Ctor, writable: true, enumerable: false, configurable: true,
      });
    });
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
    Object.defineProperty(proto, key, {
      get: brand(function () {
        var fn = slots(this)[key];
        return fn === undefined ? null : fn;
      }, 'get ' + key),
      set: brand(function (fn) {
        slots(this)[key] = typeof fn === 'function' ? fn : null;
      }, 'set ' + key),
      enumerable: true,
      configurable: true,
    });
  }

  // Expose them as NON-enumerable globals: the persona/capability modules
  // reach them, but a page walking `Object.keys(window)`/`for..in` never sees
  // them (the raw `__frot_*` syscalls' enumerability is a separate, documented
  // residual — js.md §7). Branded native so their own toString does not leak.
  [
    ['__frot_brand', brand],
    ['__frot_iface', iface],
    ['__frot_domerror', domError],
    ['__frot_onevent', onEvent],
    ['__frot_iface_seal', sealInterfaces],
    ['__frot_ifaceattrs', attrs],
    ['__frot_ifaceops', ops],
    ['__frot_slots', slots],
    ['__frot_rwattrs', rwAttrs],
  ].forEach(function (pair) {
    brand(pair[1], pair[0]);
    Object.defineProperty(g, pair[0], {
      value: pair[1],
      configurable: true,
      writable: true,
    });
  });
})(globalThis);
