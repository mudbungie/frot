// Permissions API + Notification — coherent PRESENCE without a grant (js.md
// §7/§11, identity.md §8/§10/§11, bl-1548). webbrowsertools read
// navigator.permissions and Notification; frot returned undefined — INCOHERENT,
// since Firefox 140esr exposes both, and the §10 coherence bar makes absence a
// louder tell than a costume. This masquerades the surface so feature detection
// passes, every value fixed / profile-independent (never random): a FRESH Firefox
// profile has granted nothing and prompted nothing, so permissions.query()
// resolves 'prompt' for every recognised name and Notification.permission is
// 'default'. frot shows no notification and raises no prompt — so
// requestPermission() resolves an HONEST 'default' (no grant), and a constructed
// Notification never fires an event. That no-grant / no-fire is the ONE declared
// residual (identity.md §11): honest silence, not a wrong value — a real,
// un-prompted page sees exactly this. Runs after brand.js (needs __frot_brand /
// __frot_iface), events.js (its EventTarget is the one both prototypes inherit)
// and navigator.js (extends Navigator.prototype). No syscall.
(function (g) {
  'use strict';
  var onEvent = g.__frot_onevent;
  var brand = g.__frot_brand;
  var iface = g.__frot_iface;
  var attrs = g.__frot_ifaceattrs;
  // Per-instance state lives OFF the instance (brand.js's one WeakMap), so a
  // walked PermissionStatus/Notification owns nothing — Gecko's shape (bl-3bdc).
  var slots = g.__frot_slots;

  // A branded native no-op: close()/addEventListener &c. legally do nothing —
  // there is nothing to dispatch to, so returning undefined IS the honest result.
  function noop(name) {
    return brand(function () {}, name);
  }


  // PermissionStatus and Notification ARE EventTargets, so they inherit the one
  // interface events.js owns rather than flattening three copies of its methods
  // onto each prototype — the shape Firefox has (`bl-6438`, identity.md §3.14).
  // Nothing frot does dispatches to them (the residual is silence), but a page's
  // own dispatchEvent now behaves as a real EventTarget's does.
  function inheritEventTarget(Ctor) {
    Object.setPrototypeOf(Ctor.prototype, g.EventTarget.prototype);
  }

  // --- Permissions / PermissionStatus ----------------------------------------
  // The PermissionName enum, and each name's state on a FRESH profile — both
  // READ from Firefox 153.0esr, two fresh profiles, `bl-1ab7` (identity.md
  // §3.12). A name outside the enum is not a valid enumeration value, so query()
  // REJECTS with a TypeError. The states are not uniform: `screen-wake-lock`
  // resolves **granted** un-prompted (it needs no user consent), and the earlier
  // "no name defaults 'granted', so 'prompt' across the board" was a belief the
  // measurement refuted. Every value here is fixed / profile-independent.
  var NAMES = {
    geolocation: 'prompt', notifications: 'prompt', push: 'prompt',
    'persistent-storage': 'prompt', midi: 'prompt', 'storage-access': 'prompt',
    'screen-wake-lock': 'granted', camera: 'prompt', microphone: 'prompt',
  };

  // `name` before `state`: that is the own-property order a real
  // PermissionStatus prototype enumerates in (measured, §3.12).
  var PermissionStatus = iface('PermissionStatus', ['name', 'state']);
  inheritEventTarget(PermissionStatus);
  onEvent(PermissionStatus.prototype, 'onchange');

  function status(name) {
    var s = Object.create(PermissionStatus.prototype);
    slots(s).state = NAMES[name];
    slots(s).name = name;
    return s;
  }

  // query() is a promise-returning WebIDL operation, so EVERY argument error is a
  // REJECTED promise (never a synchronous throw), matching Firefox's shape.
  var Permissions = iface('Permissions', {});
  Permissions.prototype.query = brand(function query(permission) {
    if (arguments.length < 1) {
      return Promise.reject(new TypeError(
        'Permissions.query: At least 1 argument required, but only 0 passed'));
    }
    if (permission === null || typeof permission !== 'object') {
      return Promise.reject(new TypeError(
        'Permissions.query: Argument 1 is not an object.'));
    }
    if (!('name' in permission) || permission.name === undefined) {
      return Promise.reject(new TypeError(
        "Missing required 'name' member of PermissionDescriptor."));
    }
    var name = String(permission.name);
    if (!Object.prototype.hasOwnProperty.call(NAMES, name)) {
      return Promise.reject(new TypeError(
        "Permissions.query: '" + name + "' (value of 'name' member of " +
        'PermissionDescriptor) is not a valid value for enumeration PermissionName.'));
    }
    return Promise.resolve(status(name));
  }, 'query');

  var permissions = Object.create(Permissions.prototype);
  Object.defineProperty(g.Navigator.prototype, 'permissions', {
    get: brand(function permissionsGetter() { return permissions; }, 'get permissions'),
    enumerable: true,
    configurable: true,
  });

  // --- Notification ----------------------------------------------------------
  // frot shows nothing, so `new Notification(title)` is a coherent no-op: it
  // builds a Firefox-shaped instance that displays no notification and fires no
  // event (the residual) — exactly what a real page with permission 'default'
  // sees (Firefox shows nothing until the user grants). The one synchronous check
  // is the required title argument; missing it throws TypeError, like Firefox.
  // Defaults and membership both MEASURED on Firefox 153.0esr (`bl-706b`,
  // identity.md §3.15) by constructing a real Notification: `badge`, `renotify`
  // and `timestamp` are NOT exposed at all (frot published all three), `actions`
  // is a frozen empty array (frot had none), and `silent` defaults to `false`,
  // not `null`. `timestamp` was also the one non-deterministic value in the
  // persona — it was `Date.now()`, so two runs of the same page disagreed.
  var STR = { dir: 'auto', lang: '', body: '', tag: '', icon: '' };
  var Notification = iface('Notification', null, function (inst, args) {
    if (args.length < 1) {
      throw new TypeError(
        'Notification constructor: At least 1 argument required, but only 0 passed');
    }
    var o = args[1] || {};
    var st = slots(inst);
    st.title = String(args[0]);
    Object.keys(STR).forEach(function (k) {
      st[k] = k in o ? String(o[k]) : STR[k];
    });
    st.requireInteraction = !!o.requireInteraction;
    st.silent = !!o.silent;
    st.data = 'data' in o ? o.data : null;
    st.actions = Object.freeze([]);
  });
  // Member order is Gecko's, measured: the operation, then the event handlers,
  // then the attributes — not the order the spec lists them in.
  inheritEventTarget(Notification);
  Notification.prototype.close = noop('close');
  ['onclick', 'onshow', 'onerror', 'onclose'].forEach(function (k) {
    onEvent(Notification.prototype, k);
  });
  attrs(Notification.prototype, ['title', 'dir', 'lang', 'body', 'tag', 'icon',
    'requireInteraction', 'silent', 'data', 'actions']);

  // Static surface: `permission` stays 'default' (frot prompts nothing), a native
  // accessor like Firefox's; `maxActions` is Firefox's fixed 2; requestPermission
  // resolves an honest 'default' (no grant — the residual) and calls the legacy
  // callback if given, both of Firefox's shapes.
  Object.defineProperty(Notification, 'permission', {
    get: brand(function permission() { return 'default'; }, 'get permission'),
    enumerable: true,
    configurable: true,
  });
  Object.defineProperty(Notification, 'maxActions', {
    get: brand(function maxActions() { return 2; }, 'get maxActions'),
    enumerable: true,
    configurable: true,
  });
  Notification.requestPermission = brand(function requestPermission(cb) {
    if (typeof cb === 'function') cb('default');
    return Promise.resolve('default');
  }, 'requestPermission');
})(globalThis);
