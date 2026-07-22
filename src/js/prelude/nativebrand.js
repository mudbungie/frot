// Final native-code sweep (identity.md §8, bl-3972 / bl-3926). frot's web APIs
// are implemented in the JS prelude over `__frot_*` syscalls, so an un-branded
// `fetch.toString()` / `querySelector.toString()` would disclose prelude source
// and the syscall name `frot`. This runs LAST and routes the whole existing
// surface through the ONE registry brand.js owns — no second toString wrapper,
// no per-call-site patch. Page scripts have not run yet, so every function
// reachable here is either a genuine engine built-in or a frot web API; both
// legitimately read as `[native code]`. The six later capability balls
// (canvas/WebGL/audio/indexedDB/Worker/permissions) brand their own additions
// via __frot_brand as they define them.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;

  // Brand every function-valued and accessor property directly on `obj`.
  function sweep(obj) {
    Object.getOwnPropertyNames(obj).forEach(function (k) {
      var d = Object.getOwnPropertyDescriptor(obj, k);
      if (!d) return;
      if (typeof d.value === 'function') brand(d.value, d.value.name || k);
      if (typeof d.get === 'function') brand(d.get, 'get ' + k);
      if (typeof d.set === 'function') brand(d.set, 'set ' + k);
    });
  }

  // Walk an object's own props then its prototype chain up to (not including)
  // Object.prototype — reaching the Node/Element/Document methods a page probes.
  function sweepChain(obj) {
    while (obj && obj !== Object.prototype) {
      sweep(obj);
      obj = Object.getPrototypeOf(obj);
    }
  }

  sweep(g); // window's own methods + the raw __frot_* syscalls (source hidden)
  var el = g.document && g.document.createElement('div');
  [g.document, el, g.navigator, g.screen, g.crypto, g.location, g.history].forEach(function (o) {
    if (o) sweepChain(o);
  });
})(globalThis);
