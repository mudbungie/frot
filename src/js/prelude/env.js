// Environment shims (js.md §7) — no persistence. localStorage/sessionStorage
// and document.cookie are in-memory, born empty, and die with the process;
// navigator/location are the static facts frot already has (UA it sends, final
// URL); matchMedia evaluates width queries against the fixed viewport; the rest
// are spec-legal denials (null/false/absent), never silent lies. Layered on the
// __frot_env_ua / __frot_location / __frot_viewport_width / __frot_denied
// syscalls; runs after dom.js so it can extend `document` and `Node`.
(function (g) {
  'use strict';

  // --- self/window aliasing (js.md §1 spike, subtask 8) ---------------------
  // The global *is* the window: UMD bundles probe `self`, framework/router code
  // reads `window.*`, frame-busters compare `top`/`parent` to `self`. One arena,
  // no frames (§11), so they all point at the single global.
  g.self = g;
  g.window = g;
  g.top = g;
  g.parent = g;
  g.frameElement = null;

  // --- Storage: real semantics, in-memory, born empty (§7) ------------------
  function makeStorage() {
    var map = Object.create(null);
    var api = {
      getItem: function (k) {
        k = String(k);
        return k in map ? map[k] : null;
      },
      setItem: function (k, v) {
        map[String(k)] = String(v);
      },
      removeItem: function (k) {
        delete map[String(k)];
      },
      clear: function () {
        map = Object.create(null);
      },
      key: function (i) {
        var ks = Object.keys(map);
        return i >= 0 && i < ks.length ? ks[i] : null;
      },
    };
    // A Proxy gives the bracket/dot sugar (`store.foo`, `store[k] = v`) real
    // Storage exposes, while the method names and `length` pass through.
    return new Proxy(api, {
      get: function (t, p) {
        if (p === 'length') return Object.keys(map).length;
        if (p in t) return t[p];
        return typeof p === 'string' && p in map ? map[p] : undefined;
      },
      set: function (t, p, v) {
        if (p in t) return false;
        map[String(p)] = String(v);
        return true;
      },
      has: function (t, p) {
        return p in t || (typeof p === 'string' && p in map);
      },
      deleteProperty: function (t, p) {
        delete map[String(p)];
        return true;
      },
    });
  }
  g.localStorage = makeStorage();
  g.sessionStorage = makeStorage();

  // --- document.cookie: in-memory jar, born empty (§13 OQ-3) ----------------
  // Not seeded from Set-Cookie; a jar (name->value) presented as the cookie
  // string. Setting parses the leading name=value pair and honours a past
  // expiry (max-age<=0 / an expires in the past) as a delete.
  var jar = Object.create(null);
  function expired(attrs) {
    for (var i = 1; i < attrs.length; i++) {
      var kv = attrs[i].split('=');
      var name = kv[0].trim().toLowerCase();
      if (name === 'max-age') return parseInt(kv[1], 10) <= 0;
      if (name === 'expires') return new Date(kv.slice(1).join('=')) < new Date();
    }
    return false;
  }
  Object.defineProperty(g.document, 'cookie', {
    configurable: true,
    get: function () {
      return Object.keys(jar)
        .map(function (k) {
          return k + '=' + jar[k];
        })
        .join('; ');
    },
    set: function (v) {
      var attrs = String(v).split(';');
      var pair = attrs[0].split('=');
      var name = pair[0].trim();
      if (!name) return;
      if (expired(attrs)) delete jar[name];
      else jar[name] = pair.slice(1).join('=').trim();
    },
  });

  // --- navigator: the UA frot sends, plus static facts (§7) -----------------
  var UA = g.__frot_env_ua();
  g.navigator = Object.freeze({
    userAgent: UA,
    appName: 'Netscape',
    appCodeName: 'Mozilla',
    appVersion: UA.replace(/^Mozilla\//, ''),
    product: 'Gecko',
    productSub: '20100101',
    vendor: '',
    platform: 'Linux x86_64',
    language: 'en-US',
    languages: Object.freeze(['en-US']),
    onLine: true,
    cookieEnabled: true,
    doNotTrack: null,
    // False is the truth: frot is not under WebDriver remote control, and
    // real browsers define the field (absence is itself an odd fingerprint).
    webdriver: false,
    hardwareConcurrency: 1,
    maxTouchPoints: 0,
    // Legal denial, not an exception (§6): frot reads, never submits.
    sendBeacon: function () {
      return false;
    },
    // serviceWorker / geolocation / clipboard: absent (undefined) per §7.
  });

  // --- location: the final URL; assignment is navigation = counted no-op ----
  var L = g.__frot_location();
  var loc = {
    assign: function () {
      g.__frot_denied();
    },
    replace: function () {
      g.__frot_denied();
    },
    reload: function () {
      g.__frot_denied();
    },
    toString: function () {
      return L.href;
    },
  };
  ['href', 'protocol', 'host', 'hostname', 'port', 'pathname', 'search', 'hash', 'origin'].forEach(
    function (k) {
      Object.defineProperty(loc, k, {
        enumerable: true,
        get: function () {
          return L[k];
        },
        // href/etc assignment is navigation (§7, §11) — observable, so counted.
        set: function () {
          g.__frot_denied();
        },
      });
    }
  );
  // `location = url` / `window.location = url` are navigation too (§11).
  Object.defineProperty(g, 'location', {
    configurable: true,
    get: function () {
      return loc;
    },
    set: function () {
      g.__frot_denied();
    },
  });
  Object.defineProperty(g.document, 'location', {
    configurable: true,
    get: function () {
      return loc;
    },
    set: function () {
      g.__frot_denied();
    },
  });

  // --- history: in-memory, no navigation (§7) -------------------------------
  // Routers read `history.state` on first render; absent, they throw. pushState/
  // replaceState set `.state` (the only fact read back); pushState also bumps
  // `length`. They do NOT mutate `location`: it stays the honest fetched URL
  // (§7, "location is a static fact") — frot takes ONE impression, and SPA
  // routers pick their initial route from that location. go/back/forward are
  // no-ops (there is nowhere to go). Born fresh, discarded at exit (§7).
  g.history = {
    state: null,
    length: 1,
    scrollRestoration: 'auto',
    pushState: function (state, title, url) {
      this.state = state;
      this.length += 1;
    },
    replaceState: function (state, title, url) {
      this.state = state;
    },
    go: function () {},
    back: function () {},
    forward: function () {},
  };

  // --- viewport & width facts (§7/§8) ---------------------------------------
  // A fixed 1280×720 viewport (layout.rs VIEWPORT_WIDTH/VIEWPORT_HEIGHT). Pages
  // gate desktop chrome on width — mdbook reads window.innerWidth, and a NaN
  // compare takes the mobile branch and strips its sidebar — so the widths frot
  // already knows are exposed honestly. window inner == outer (no browser
  // chrome). Element clientWidth/clientHeight == offsetWidth/offsetHeight
  // (borderless, scrollbar-less model, §8). The documentElement reports the
  // viewport itself, as browsers do — the primary way a page reads viewport size.
  var VW = g.__frot_viewport_width();
  var VH = g.__frot_viewport_height();
  g.innerWidth = g.outerWidth = VW;
  g.innerHeight = g.outerHeight = VH;
  if (g.Node) {
    Object.defineProperty(g.Node.prototype, 'clientWidth', {
      get: function () {
        var de = g.document.documentElement;
        return de && this._id === de._id ? VW : this.offsetWidth;
      },
    });
    Object.defineProperty(g.Node.prototype, 'clientHeight', {
      get: function () {
        var de = g.document.documentElement;
        return de && this._id === de._id ? VH : this.offsetHeight;
      },
    });
  }

  // --- matchMedia: width queries vs the fixed viewport; else never (§7) -----
  function evalWidth(query) {
    var re = /\((min-width|max-width|width)\s*:\s*(\d+(?:\.\d+)?)(px|em|rem)\)/g;
    var m,
      result = null;
    while ((m = re.exec(query))) {
      // em/rem convert at a 16px root font-size (so 80em == the 1280px viewport).
      var px = m[3] === 'px' ? +m[2] : +m[2] * 16;
      var pass = m[1] === 'min-width' ? VW >= px : m[1] === 'max-width' ? VW <= px : VW === px;
      result = result === null ? pass : result && pass;
    }
    // No width feature -> "everything else matches never" (§7).
    return result === null ? false : result;
  }
  g.matchMedia = function (query) {
    query = String(query);
    return {
      matches: evalWidth(query),
      media: query,
      onchange: null,
      addListener: function () {},
      removeListener: function () {},
      addEventListener: function () {},
      removeEventListener: function () {},
      dispatchEvent: function () {
        return false;
      },
    };
  };

  // --- canvas getContext: spec-legal null, not an exception (§7) ------------
  if (g.Node) {
    g.Node.prototype.getContext = function () {
      return this.tagName === 'CANVAS' ? null : undefined;
    };
  }
  // indexedDB, Worker, WebSocket, EventSource, WebAssembly, navigator.service-
  // Worker: never defined -> `typeof` is 'undefined', so feature detection
  // falls through (§6, §7). Absence is the denial; no code lies otherwise.
})(globalThis);
