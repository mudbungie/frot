// navigator — the pinned Firefox ESR identity surface, every fact derived from
// the one __frot_env_profile() channel (identity.md §4/§8, bl-3972). No literal
// here duplicates an HTTP fact: userAgent and language(s) ride the same
// effective UA / Accept-Language the transport sent, so wire and JS cannot
// disagree (the UA-coherence fix). appVersion does NOT: Gecko freezes it at
// `5.0 (X11)` whatever the UA says, so it rides the profile (bl-6491).
// Firefox-shaped: a real Navigator constructor,
// @@toStringTag, and every property an enumerable accessor on Navigator.prototype
// (not an own data prop) — brand/prototype/descriptor probes pass, not just
// values. Runs after brand.js (needs __frot_iface) and dom.js (extends document).
(function (g) {
  'use strict';
  var P = JSON.parse(g.__frot_env_profile());

  // --- plugins / mimeTypes: the uniform Gecko PDF-viewer shim set --------------
  // Modern browsers report an identical 5-plugin / 2-mimeType PDF set for anti-
  // fingerprinting; frot mirrors it. Backing data lives in brand.js's one
  // instance-state WeakMap, so a page reading `mimeTypes[0]` sees only the spec
  // surface — non-enumerable own slots were still own properties, and
  // `Object.getOwnPropertyNames` reported every one (bl-3bdc, §3.16).
  var slots = g.__frot_slots;
  function hidden(o, k, v) {
    Object.defineProperty(o, k, { value: v, configurable: true });
  }
  var MimeType = g.__frot_iface('MimeType',
    ['type', 'suffixes', 'description', 'enabledPlugin']);
  var Plugin = g.__frot_iface('Plugin', {
    name: null,
    filename: null,
    description: null,
    version: function () { return null; },
    length: function () { return slots(this).mimes.length; },
  });

  // A branded array-like collection (PluginArray / MimeTypeArray): fixed items,
  // integer-indexed + name-keyed, with native item()/namedItem(). @@toStringTag
  // gives `[object <tag>]`; length is a prototype accessor, as in Firefox.
  function collection(tag, items, key) {
    var Ctor = g.__frot_iface(tag, { length: function () { return items.length; } });
    var arr = Object.create(Ctor.prototype);
    items.forEach(function (it, i) {
      Object.defineProperty(arr, i, { value: it, enumerable: true, configurable: true });
      hidden(arr, key(it), it);
    });
    Ctor.prototype.item = g.__frot_brand(function item(i) { return items[i >>> 0] || null; });
    Ctor.prototype.namedItem = g.__frot_brand(function namedItem(n) {
      return Object.prototype.hasOwnProperty.call(arr, n) ? arr[n] : null;
    });
    return arr;
  }

  var mimes = [['application/pdf', 'pdf'], ['text/pdf', 'pdf']].map(function (m) {
    var mt = Object.create(MimeType.prototype);
    var st = slots(mt);
    st.type = m[0];
    st.suffixes = m[1];
    st.description = 'Portable Document Format';
    return mt;
  });
  var pluginItems = [
    'PDF Viewer', 'Chrome PDF Viewer', 'Chromium PDF Viewer',
    'Microsoft Edge PDF Viewer', 'WebKit built-in PDF',
  ].map(function (name) {
    var pl = Object.create(Plugin.prototype);
    var st = slots(pl);
    st.name = name;
    st.filename = 'internal-pdf-viewer';
    st.description = 'Portable Document Format';
    st.mimes = mimes;
    mimes.forEach(function (mt, i) {
      Object.defineProperty(pl, i, { value: mt, enumerable: true, configurable: true });
    });
    return pl;
  });
  var pluginArray = collection('PluginArray', pluginItems, function (p) { return p.name; });
  var mimeArray = collection('MimeTypeArray', mimes, function (m) { return m.type; });
  mimes.forEach(function (mt) { slots(mt).enabledPlugin = pluginItems[0]; });

  // --- Navigator: every fact an accessor on the prototype (Firefox shape) -------
  // languages is a frozen array (Firefox reports it immutable), one copy shared.
  var LANGS = Object.freeze(P.languages.slice());
  var Navigator = g.__frot_iface('Navigator', {
    userAgent: function () { return P.userAgent; },
    appVersion: function () { return P.appVersion; },
    appName: function () { return P.appName; },
    appCodeName: function () { return P.appCodeName; },
    product: function () { return P.product; },
    productSub: function () { return P.productSub; },
    vendor: function () { return P.vendor; },
    vendorSub: function () { return P.vendorSub; },
    platform: function () { return P.platform; },
    oscpu: function () { return P.oscpu; },
    language: function () { return P.language; },
    languages: function () { return LANGS; },
    onLine: function () { return P.onLine; },
    cookieEnabled: function () { return P.cookieEnabled; },
    doNotTrack: function () { return P.doNotTrack; },
    // Truthful: frot is under no WebDriver remote control. Real Firefox defines
    // the field false (absence is itself an odd fingerprint) — identity.md §8.
    webdriver: function () { return false; },
    hardwareConcurrency: function () { return P.hardwareConcurrency; },
    maxTouchPoints: function () { return P.maxTouchPoints; },
    buildID: function () { return P.buildID; },
    pdfViewerEnabled: function () { return P.pdfViewerEnabled; },
    plugins: function () { return pluginArray; },
    mimeTypes: function () { return mimeArray; },
  });
  Navigator.prototype.javaEnabled = g.__frot_brand(function javaEnabled() { return false; });
  // A legal denial, never a submission (js.md §6): frot reads, it never POSTs.
  Navigator.prototype.sendBeacon = g.__frot_brand(function sendBeacon() { return false; });

  var navigator = Object.create(Navigator.prototype);
  Object.defineProperty(g, 'navigator', {
    get: function () { return navigator; },
    configurable: true,
    enumerable: true,
  });
})(globalThis);
