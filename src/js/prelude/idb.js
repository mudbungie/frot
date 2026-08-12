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

  function noop(name) {
    return brand(function () {}, name);
  }

  // The EventTarget methods IDBRequest inherits: present and native. Nothing frot
  // does ever dispatches to a request, so retaining listeners would be dead state;
  // dispatchEvent returns true (not canceled), matching EventTarget's contract.
  function eventTarget(proto) {
    proto.addEventListener = noop('addEventListener');
    proto.removeEventListener = noop('removeEventListener');
    proto.dispatchEvent = brand(function () { return true; }, 'dispatchEvent');
  }

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
  function keyRank(k) {
    if (typeof k === 'number' && !isNaN(k)) return 0;
    if (k instanceof Date && !isNaN(k.getTime())) return 1;
    if (typeof k === 'string') return 2;
    if (Array.isArray(k)) return 3;
    throw domError('DataError',
      'Data provided to an operation does not meet requirements.');
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
  var IDBRequest = iface('IDBRequest', {});
  eventTarget(IDBRequest.prototype);
  constGetter(IDBRequest.prototype, 'readyState', 'pending');
  constGetter(IDBRequest.prototype, 'source', null);
  constGetter(IDBRequest.prototype, 'transaction', null);
  pendingThrows(IDBRequest.prototype, 'result');
  pendingThrows(IDBRequest.prototype, 'error');
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
    if (arguments.length < 1) {
      throw new TypeError('IDBFactory.open: At least 1 argument required, but only 0 passed');
    }
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
    if (arguments.length < 1) {
      throw new TypeError('IDBFactory.deleteDatabase: At least 1 argument required, but only 0 passed');
    }
    return openRequest();
  }, 'deleteDatabase');
  IDBFactory.prototype.databases = brand(function () {
    return Promise.resolve([]);
  }, 'databases');
  IDBFactory.prototype.cmp = brand(function (a, b) {
    if (arguments.length < 2) {
      throw new TypeError('IDBFactory.cmp: At least 2 arguments required, but only ' + arguments.length + ' passed');
    }
    return cmpKeys(a, b);
  }, 'cmp');

  var indexedDB = Object.create(IDBFactory.prototype);
  Object.defineProperty(g, 'indexedDB', {
    value: indexedDB,
    configurable: true,
    enumerable: true,
    writable: true,
  });

  // --- The rest of the zoo: pure presence, non-constructable (Firefox throws
  // "Illegal constructor" for `new IDBDatabase()` &c.). IDBCursorWithValue
  // inherits IDBCursor; the shapes exist so feature detection reads them native.
  ['IDBDatabase', 'IDBTransaction', 'IDBObjectStore', 'IDBIndex', 'IDBCursor',
    'IDBCursorWithValue', 'IDBKeyRange'].forEach(function (nm) {
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
