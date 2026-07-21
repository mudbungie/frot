// Capability-surface probe instrumentation (bl-bd4e). Loaded ONLY under the
// `probe::measure` instrument, after the shipping prelude and before any page
// script. Its one job is to RECORD which fingerprinting / feature-detection
// surfaces a page touches, via the __frot_probe(name) syscall, while returning
// EXACTLY what frot returns today (the same null/undefined/value/throw). It
// never fabricates a capability (identity.md §10): a wrapped absent surface
// stays absent in value, so the measured probe path is the path a real `--js`
// run takes. The only observable perturbation is that a wrapped absent global's
// name becomes present to `in`/hasOwnProperty (not to `typeof`, truthiness, or a
// bare reference) — a documented measurement artifact, noted in the writeup.
(function (g) {
  'use strict';

  function record(name) {
    g.__frot_probe(String(name));
  }

  // --- Absent/present feature-detect globals --------------------------------
  // For each watched global: capture its current value (undefined if absent) and
  // redefine it as a recording getter that returns that same value. Reading it —
  // `if (window.X)`, `typeof X`, a bare `X` reference — records the probe and
  // yields frot's real answer unchanged. Skip a non-configurable global (cannot
  // wrap without breaking it); absent ones define fresh.
  var GLOBALS = [
    'AudioContext', 'OfflineAudioContext', 'webkitAudioContext',
    'Worker', 'SharedWorker', 'ServiceWorker',
    'WebAssembly',
    'RTCPeerConnection', 'webkitRTCPeerConnection', 'RTCDataChannel',
    'WebGLRenderingContext', 'WebGL2RenderingContext',
    'WebSocket', 'EventSource',
    'indexedDB', 'BroadcastChannel', 'Notification',
    'crypto', 'performance', 'Intl',
    'screen', 'devicePixelRatio',
    'SharedArrayBuffer', 'GPU', 'Bluetooth', 'USB',
    'chrome', 'speechSynthesis', 'Gamepad',
  ];
  GLOBALS.forEach(function (name) {
    var desc = Object.getOwnPropertyDescriptor(g, name);
    if (desc && !desc.configurable) return;
    var orig = g[name];
    Object.defineProperty(g, name, {
      configurable: true,
      get: function () {
        record(name);
        return orig;
      },
    });
  });

  // --- navigator: a recording Proxy over the frozen shim ---------------------
  // navigator is Object.freeze()d by env.js, so it cannot take getters. A Proxy
  // wraps it: reads of a watchlisted fingerprint property are recorded, then
  // forwarded to the real navigator (returning its frozen value, or undefined
  // for an absent property — the frozen-target invariant holds because the trap
  // returns the exact target value). Non-watchlisted reads forward silently, so
  // `navigator.userAgent` (read by nearly every page) is not counted as a probe.
  var NAV = {
    plugins: 1, mimeTypes: 1, mediaDevices: 1, hardwareConcurrency: 1,
    deviceMemory: 1, userAgentData: 1, connection: 1, getBattery: 1,
    permissions: 1, webdriver: 1, languages: 1, doNotTrack: 1, vendor: 1,
    vendorSub: 1, oscpu: 1, buildID: 1, product: 1, productSub: 1,
    platform: 1, pdfViewerEnabled: 1, maxTouchPoints: 1, queryLocalFonts: 1,
    gpu: 1, bluetooth: 1, usb: 1, credentials: 1, storage: 1, sendBeacon: 1,
  };
  var realNav = g.navigator;
  var navProxy = new Proxy(realNav, {
    get: function (t, p) {
      if (typeof p === 'string' && NAV[p]) record('navigator.' + p);
      return t[p];
    },
  });
  Object.defineProperty(g, 'navigator', {
    configurable: true,
    get: function () {
      return navProxy;
    },
  });

  // --- canvas.getContext: record the requested context type ------------------
  // getContext already returns null (env.js, spec-legal). The requested type is
  // the branch signal: `2d` is canvas fingerprinting, `webgl`/`webgl2` is WebGL
  // fingerprinting. WebGL getParameter / getExtension / debug-renderer strings
  // are unreachable behind the null context, so getContext(type) is the correct,
  // sufficient WebGL probe. Behaviour preserved: still returns the original.
  if (g.Node) {
    var realGetContext = g.Node.prototype.getContext;
    g.Node.prototype.getContext = function (type) {
      if (this.tagName === 'CANVAS') {
        record('canvas.getContext(' + String(type == null ? '' : type).toLowerCase() + ')');
      }
      return realGetContext.apply(this, arguments);
    };
  }

  // --- document.fonts: font-enumeration surface ------------------------------
  // Absent in frot; a recording getter returns undefined (preserved). Note the
  // limitation recorded in the writeup: offsetWidth-based font *measurement*
  // loops are indistinguishable from ordinary layout reads and are NOT captured
  // here — only the FontFaceSet API is.
  if (g.document) {
    var fontsDesc = Object.getOwnPropertyDescriptor(g.document, 'fonts');
    if (!fontsDesc || fontsDesc.configurable) {
      var origFonts = g.document.fonts;
      Object.defineProperty(g.document, 'fonts', {
        configurable: true,
        get: function () {
          record('document.fonts');
          return origFonts;
        },
      });
    }
  }

  // --- timezone probing: Date.prototype.getTimezoneOffset --------------------
  var realTZ = Date.prototype.getTimezoneOffset;
  Date.prototype.getTimezoneOffset = function () {
    record('Date.getTimezoneOffset');
    return realTZ.apply(this, arguments);
  };

  // --- prototype-shape / descriptor probing ----------------------------------
  // Anti-bot code inspects the SHAPE of identity objects — reading the property
  // descriptor of `navigator.webdriver`, or the prototype chain of navigator /
  // window — to detect shimming. Record only when the target is one of frot's
  // identity objects, so ordinary library use of these reflection methods (on
  // arbitrary objects) is not counted. Forward the real result unchanged.
  function isIdentity(t) {
    return t === navProxy || t === realNav || t === g || t === g.document;
  }
  var realGOPD = Object.getOwnPropertyDescriptor;
  Object.getOwnPropertyDescriptor = function (t, p) {
    if (isIdentity(t)) record('Object.getOwnPropertyDescriptor(identity)');
    return realGOPD.apply(this, arguments);
  };
  var realGPO = Object.getPrototypeOf;
  Object.getPrototypeOf = function (t) {
    if (isIdentity(t)) record('Object.getPrototypeOf(identity)');
    return realGPO.apply(this, arguments);
  };

  // --- Function.prototype.toString: native-code inspection -------------------
  // Fingerprinters call fn.toString() to see whether a "native" function was
  // monkeypatched (a shim reveals JS source instead of `[native code]`).
  // Aggregate — it also fires on ordinary library code, so the writeup reads it
  // as a weak signal — but recording it is honest data. Behaviour preserved.
  var realFTS = Function.prototype.toString;
  Function.prototype.toString = function () {
    record('Function.prototype.toString');
    return realFTS.apply(this, arguments);
  };
})(globalThis);
