// IndexedDB — coherent IDBFactory + interface-zoo PRESENCE without persistence
// (js.md §7/§11, identity.md §8/§10/§11, bl-8dde). Firefox exposes `indexedDB`
// and the IDB* zoo; the §10 coherence bar makes absence a louder tell than a
// costume, so feature detection must pass: `'indexedDB' in window`, `indexedDB
// instanceof IDBFactory`, `typeof indexedDB.open === 'function'`, branded-native,
// Firefox-shaped prototypes. frot is stateless (VISION §1: no persistence), so
// there is NO backing store: `open()`/`deleteDatabase()` return a real,
// Firefox-shaped, permanently-`pending` IDBOpenDBRequest whose onsuccess/
// onupgradeneeded/onerror NEVER fire. That non-completion is the ONE declared
// residual (identity.md §11): honest silence, not a wrong value — and it is LESS
// detectable than firing an error, because a real Firefox `open()` of a fresh DB
// SUCCEEDS, so an error callback would CONTRADICT the persona (a louder tell).
// `databases()` honestly resolves to [] (a stateless store has none); `cmp()` is
// a real synchronous key comparison. Every value is fixed / profile-independent;
// no syscall (pure JS over brand.js + loop.js's `g.Event`, so it runs after both).
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var iface = g.__frot_iface;

  // A DOMException-shaped error: a plain error carrying the spec `name` (quickjs
  // lacks DOMException) — brand.js's one maker (bl-273b).
  var domError = g.__frot_domerror;

  // An event-handler IDL accessor (onsuccess/onupgradeneeded/…): enumerable,
  // native, defaults null, settable — but never fires (silence is the residual).
  // brand.js's one maker; this file carried a fourth private copy until `bl-6438`
  // deleted it, the last of the three `bl-1ab7` set out to collapse.
  var onEvent = g.__frot_onevent;

  // A native prototype getter returning a fixed value.
  function constGetter(proto, key, value) {
    Object.defineProperty(proto, key, {
      get: brand(function () { return value; }, 'get ' + key),
      enumerable: true,
      configurable: true,
    });
  }

  // A native prototype getter that throws InvalidStateError — Firefox's `result`
  // and `error` getters do exactly this while `readyState` is 'pending', which
  // ours permanently is (the request never completes: the residual).
  function pendingThrows(proto, key) {
    Object.defineProperty(proto, key, {
      get: brand(function () {
        throw domError('InvalidStateError',
          'An attempt was made to use an object that is not, or is no longer, usable');
      }, 'get ' + key),
      enumerable: true,
      configurable: true,
    });
  }

  // --- IDB key comparison (IDBFactory.cmp): a real, coherent, synchronous sort
  // over the standard key types, ranked number < Date < string < array, matching
  // the spec's ordering. An invalid key throws DataError, exactly as Firefox does.
  // Gecko's generic DataError text, measured (`bl-1ab7`, §3.12) — one maker,
  // because every DataError in this file is the same one.
  function dataError() {
    return domError('DataError',
      'Data provided to an operation does not meet requirements.');
  }

  // Gecko's WebIDL arity TypeError. The wording was READ off `IDBFactory.open`
  // (`bl-1ab7`); only the method name and the counts vary between operations, so
  // it is written once here. The per-method string was not itself measured.
  function requireArgs(method, args, n) {
    if (args.length < n) {
      throw new TypeError(method + ': At least ' + n + ' argument'
        + (n === 1 ? '' : 's') + ' required, but only ' + args.length + ' passed');
    }
  }

  function keyRank(k) {
    if (typeof k === 'number' && !isNaN(k)) return 0;
    if (k instanceof Date && !isNaN(k.getTime())) return 1;
    if (typeof k === 'string') return 2;
    if (Array.isArray(k)) return 3;
    throw dataError();
  }
  function cmpKeys(a, b) {
    var ra = keyRank(a);
    var rb = keyRank(b);
    if (ra !== rb) return ra < rb ? -1 : 1;
    if (ra === 3) {
      var n = Math.min(a.length, b.length);
      for (var i = 0; i < n; i++) {
        var c = cmpKeys(a[i], b[i]);
        if (c !== 0) return c;
      }
      return a.length === b.length ? 0 : a.length < b.length ? -1 : 1;
    }
    var x = ra === 1 ? a.getTime() : a;
    var y = ra === 1 ? b.getTime() : b;
    return x < y ? -1 : x > y ? 1 : 0;
  }

  // --- IDBRequest / IDBOpenDBRequest: the shape `open()` hands out -------------
  // An IDBRequest IS an EventTarget: it inherits the one interface events.js owns
  // rather than flattening three copies of its methods (`bl-6438`, §3.14). The
  // request never completes, so frot dispatches nothing to it — but a page's own
  // dispatchEvent behaves as a real EventTarget's does.
  var IDBRequest = iface('IDBRequest', {});
  Object.setPrototypeOf(IDBRequest.prototype, g.EventTarget.prototype);
  // Attribute order is Gecko's own, measured (identity.md §3.15): result, error,
  // source, transaction, readyState — not the order the spec text lists them in.
  pendingThrows(IDBRequest.prototype, 'result');
  pendingThrows(IDBRequest.prototype, 'error');
  constGetter(IDBRequest.prototype, 'source', null);
  constGetter(IDBRequest.prototype, 'transaction', null);
  constGetter(IDBRequest.prototype, 'readyState', 'pending');
  onEvent(IDBRequest.prototype, 'onsuccess');
  onEvent(IDBRequest.prototype, 'onerror');

  var IDBOpenDBRequest = iface('IDBOpenDBRequest', {});
  Object.setPrototypeOf(IDBOpenDBRequest.prototype, IDBRequest.prototype);
  onEvent(IDBOpenDBRequest.prototype, 'onblocked');
  onEvent(IDBOpenDBRequest.prototype, 'onupgradeneeded');

  function openRequest() {
    return Object.create(IDBOpenDBRequest.prototype);
  }

  // --- IDBFactory: open/deleteDatabase (pending, never fire), databases (empty),
  // cmp (real). Firefox throws TypeError for a missing name or an illegal version.
  var IDBFactory = iface('IDBFactory', {});
  IDBFactory.prototype.open = brand(function (name, version) {
    requireArgs('IDBFactory.open', arguments, 1);
    // Three distinct WebIDL failures, each message READ off Firefox 153.0esr
    // (`bl-1ab7`, §3.12) rather than paraphrased: a non-finite version, a
    // negative one (unsigned long long range), and zero. `1.5` is NOT an error —
    // it coerces to 1 — so no integrality check belongs here.
    if (version !== undefined) {
      var v = Number(version);
      if (!isFinite(v)) {
        throw new TypeError('IDBFactory.open: Argument 2 is not a finite value, '
          + 'so is out of range for unsigned long long.');
      }
      if (v < 0) {
        throw new TypeError(
          'IDBFactory.open: Argument 2 is out of range for unsigned long long.');
      }
      if (Math.floor(v) === 0) {
        throw new TypeError('IDBFactory.open: 0 (Zero) is not a valid database version.');
      }
    }
    return openRequest();
  }, 'open');
  IDBFactory.prototype.deleteDatabase = brand(function (name) {
    requireArgs('IDBFactory.deleteDatabase', arguments, 1);
    return openRequest();
  }, 'deleteDatabase');
  IDBFactory.prototype.databases = brand(function () {
    return Promise.resolve([]);
  }, 'databases');
  IDBFactory.prototype.cmp = brand(function (a, b) {
    requireArgs('IDBFactory.cmp', arguments, 2);
    return cmpKeys(a, b);
  }, 'cmp');

  var indexedDB = Object.create(IDBFactory.prototype);
  Object.defineProperty(g, 'indexedDB', {
    value: indexedDB,
    configurable: true,
    enumerable: true,
    writable: true,
  });

  // --- IDBKeyRange: the one IDB interface that needs no store -----------------
  // A key range is a pure VALUE object — two bounds and two open/closed flags —
  // so unlike the rest of the zoo it can be built completely and honestly, with
  // no residual at all. It was presence-only, so `IDBKeyRange.bound(1, 2)` threw
  // "Illegal constructor" where Firefox returns an `[object IDBKeyRange]`.
  // Measured on 153.0esr (`bl-6438`, identity.md §3.14):
  //   Object.getOwnPropertyNames(IDBKeyRange)
  //     === ["length","name","prototype","only","lowerBound","upperBound","bound"]
  //   Object.keys(Object.getPrototypeOf(IDBKeyRange.bound(1,2)))
  //     === ["includes","lower","upper","lowerOpen","upperOpen"]
  // Definition order below IS those two orders — `includes` before the four
  // accessors, the four statics after the constructor's own length/name/prototype.
  // The bounds live in a WeakMap, so an instance owns nothing a page can walk,
  // exactly like a real one.
  var IDBKeyRange = iface('IDBKeyRange', {});
  var bounds = new WeakMap();
  function makeRange(lower, upper, lowerOpen, upperOpen) {
    var r = Object.create(IDBKeyRange.prototype);
    bounds.set(r, {
      lower: lower, upper: upper, lowerOpen: lowerOpen, upperOpen: upperOpen,
    });
    return r;
  }
  IDBKeyRange.prototype.includes = brand(function (key) {
    requireArgs('IDBKeyRange.includes', arguments, 1);
    keyRank(key);
    var b = bounds.get(this);
    var lo = b.lower === undefined ? null : cmpKeys(key, b.lower);
    var hi = b.upper === undefined ? null : cmpKeys(key, b.upper);
    if (lo !== null && (lo < 0 || (lo === 0 && b.lowerOpen))) return false;
    return !(hi !== null && (hi > 0 || (hi === 0 && b.upperOpen)));
  }, 'includes');
  ['lower', 'upper', 'lowerOpen', 'upperOpen'].forEach(function (k) {
    Object.defineProperty(IDBKeyRange.prototype, k, {
      get: brand(function () { return bounds.get(this)[k]; }, 'get ' + k),
      enumerable: true,
      configurable: true,
    });
  });
  // Each static's signature carries only its REQUIRED arguments, so `.length` is
  // the WebIDL one (optional arguments do not count toward it); the optional
  // flags are read off `arguments`. The lengths themselves were not measured.
  IDBKeyRange.only = brand(function (value) {
    requireArgs('IDBKeyRange.only', arguments, 1);
    keyRank(value);
    return makeRange(value, value, false, false);
  }, 'only');
  IDBKeyRange.lowerBound = brand(function (lower) {
    requireArgs('IDBKeyRange.lowerBound', arguments, 1);
    keyRank(lower);
    return makeRange(lower, undefined, !!arguments[1], true);
  }, 'lowerBound');
  IDBKeyRange.upperBound = brand(function (upper) {
    requireArgs('IDBKeyRange.upperBound', arguments, 1);
    keyRank(upper);
    return makeRange(undefined, upper, true, !!arguments[1]);
  }, 'upperBound');
  IDBKeyRange.bound = brand(function (lower, upper) {
    requireArgs('IDBKeyRange.bound', arguments, 2);
    var lowerOpen = !!arguments[2];
    var upperOpen = !!arguments[3];
    // Two DataError cases, both READ off Firefox 153.0esr (`bl-706b`, §3.15):
    // a reversed range, and equal bounds with BOTH ends open — which describes
    // an empty range and Gecko rejects. Equal bounds with either end closed is
    // fine (`bound(1, 1)` returns a range), so the check is not `>=`.
    if (cmpKeys(lower, upper) > 0) throw dataError();
    if (cmpKeys(lower, upper) === 0 && lowerOpen && upperOpen) throw dataError();
    return makeRange(lower, upper, lowerOpen, upperOpen);
  }, 'bound');

  // --- The rest of the zoo: pure presence, non-constructable (Firefox throws
  // "Illegal constructor" for `new IDBDatabase()` &c.). IDBCursorWithValue
  // inherits IDBCursor; the shapes exist so feature detection reads them native.
  ['IDBDatabase', 'IDBTransaction', 'IDBObjectStore', 'IDBIndex', 'IDBCursor',
    'IDBCursorWithValue'].forEach(function (nm) {
    iface(nm, {});
  });
  Object.setPrototypeOf(g.IDBCursorWithValue.prototype, g.IDBCursor.prototype);

  // IDBVersionChangeEvent IS constructable in Firefox (an Event subtype), UNLIKE
  // the interface zoo; inherit loop.js's `g.Event` so `instanceof Event` holds.
  var VCE = brand(function (type, init) {
    if (arguments.length < 1) {
      throw new TypeError('IDBVersionChangeEvent constructor: At least 1 argument required, but only 0 passed');
    }
    g.Event.call(this, type, init);
    this.oldVersion = init && init.oldVersion ? init.oldVersion : 0;
    this.newVersion = init && init.newVersion !== undefined ? init.newVersion : null;
  }, 'IDBVersionChangeEvent');
  VCE.prototype = Object.create(g.Event.prototype);
  Object.defineProperty(VCE.prototype, 'constructor', {
    value: VCE,
    configurable: true,
    writable: true,
  });
  Object.defineProperty(VCE.prototype, Symbol.toStringTag, {
    value: 'IDBVersionChangeEvent',
    configurable: true,
  });
  Object.defineProperty(g, 'IDBVersionChangeEvent', {
    value: VCE,
    configurable: true,
    writable: true,
  });
})(globalThis);
