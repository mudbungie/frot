// Element property breadth the frameworks exercise (js.md §3): the `style`
// facade, the reflected attributes (`id`, `className`, and the tag-specific
// `rel`/`href`/`src`), and `dataset`, each on the interface the binary puts it
// on (bl-78cb). No cached state — every read/write is a fresh syscall against
// the one arena, js.md §2. Loads after dom.js; the Element methods are in
// elem2.js (kept split under the 300-line cap).
(function (g) {
  'use strict';
  var proto = g.Element.prototype;
  var tagdef = g.__frot_tagdef;
  // `style`, `dataset`, `focus` and `blur` are on HTMLElement.prototype and,
  // separately, on SVGElement.prototype — not on Element (measured).
  var STYLED = [g.HTMLElement.prototype, g.SVGElement.prototype];

  // --- inline style, backed by the `style` attribute (single source of truth) -
  // A CSSStyleDeclaration-like Proxy so `'display' in el.style` feature-detects
  // (ReactDOM probes supported properties this way) instead of throwing on an
  // undefined `.style`. Reads/writes reparse/serialize the attribute, so the
  // arena stays the one representation.
  function parseDecls(str) {
    var m = Object.create(null);
    String(str || '')
      .split(';')
      .forEach(function (decl) {
        var i = decl.indexOf(':');
        if (i < 0) return;
        var prop = decl.slice(0, i).trim().toLowerCase();
        if (prop) m[prop] = decl.slice(i + 1).trim();
      });
    return m;
  }
  function serialize(m) {
    return Object.keys(m)
      .map(function (k) {
        return k + ': ' + m[k];
      })
      .join('; ');
  }
  function dash(name) {
    return String(name)
      .replace(/^(webkit|moz|ms|o)([A-Z])/, '-$1$2')
      .replace(/[A-Z]/g, function (c) {
        return '-' + c.toLowerCase();
      });
  }
  function styleFor(el) {
    function read() {
      return parseDecls(el.getAttribute('style'));
    }
    function write(m) {
      var s = serialize(m);
      if (s) el.setAttribute('style', s);
      else el.removeAttribute('style');
    }
    var api = {
      getPropertyValue: function (p) {
        return read()[String(p).toLowerCase()] || '';
      },
      setProperty: function (p, v) {
        var m = read();
        m[String(p).toLowerCase()] = String(v);
        write(m);
      },
      removeProperty: function (p) {
        var m = read();
        var k = String(p).toLowerCase();
        var old = m[k] || '';
        delete m[k];
        write(m);
        return old;
      },
      get cssText() {
        return el.getAttribute('style') || '';
      },
      set cssText(v) {
        if (v) el.setAttribute('style', String(v));
        else el.removeAttribute('style');
      },
    };
    return new Proxy(api, {
      get: function (t, p) {
        if (p in t) return t[p];
        if (typeof p !== 'string') return undefined;
        return read()[dash(p)] || '';
      },
      set: function (t, p, v) {
        if (p === 'cssText') {
          // [LegacyNullToEmptyString]: `el.style = null` clears the declaration
          // in a browser, it does not write the string "null" (Chrome 139).
          t.cssText = v == null ? '' : String(v);
          return true;
        }
        if (typeof p !== 'string' || typeof t[p] === 'function') return true;
        var m = read();
        var k = dash(p);
        if (v === '' || v == null) delete m[k];
        else m[k] = String(v);
        write(m);
        return true;
      },
      has: function (t, p) {
        if (p in t) return true;
        return typeof p === 'string' && dash(p) in read();
      },
    });
  }
  // --- [PutForwards] is delegation, not reflection (bl-273b) -----------------
  // WebIDL `[PutForwards=cssText] readonly attribute CSSStyleDeclaration style`
  // does not make `style` a settable field: `el.style = 'color:red'` is DEFINED
  // to run `el.style.cssText = 'color:red'`, and `el.classList = 'a b'` to run
  // `el.classList.value = 'a b'` (Chrome 139: both land on the attribute, and
  // the object the getter returns is unchanged). So this is NOT `reflectString`
  // — nothing here reflects an attribute of its own name. The maker takes the
  // getter and the one property the write forwards to, and the fact still has
  // exactly one home: the attribute the forwarded-to object is a view over.
  // Getter-only was the bug: since bl-0679 classic scripts are sloppy, so the
  // assignment was a silent no-op (and a TypeError in a module) — jQuery's
  // `.css()` fast path and hand-written `el.style = ...` both simply vanished.
  function putForwards(get, prop) {
    return {
      configurable: true,
      get: get,
      set: function (v) {
        var target = get.call(this);
        // `relList` is honestly undefined off link/a/area/form (tokenlist.js):
        // where a browser has no such accessor at all there is nothing to
        // forward to, so the write lands nowhere in either.
        if (target) target[prop] = v;
      },
    };
  }
  // Shared with tokenlist.js's classList/relList over the same non-enumerable
  // seam as `__frot_reflect` — one rule for the whole [PutForwards] family.
  Object.defineProperty(g, '__frot_forwards', {
    value: g.__frot_brand(putForwards, '__frot_forwards'),
    configurable: true,
    writable: true,
  });
  STYLED.forEach(function (P) {
    Object.defineProperty(
      P,
      'style',
      putForwards(function () {
        return styleFor(this);
      }, 'cssText')
    );
  });

  // --- Reflected attributes ARE accessors (bl-d313) -------------------------
  // A reflected DOM-string property is not a field over the attribute, it IS
  // the attribute: the getter reads it ('' when absent, as WebIDL requires),
  // the setter writes it through the very syscall `setAttribute` uses. So
  // `el.id = x` and `el.setAttribute('id', x)` are indistinguishable —
  // `getElementById`, selectors, the views' serialization, and the generation
  // counter that invalidates the §8 style/layout cache all see it at once,
  // because there is still exactly one place the fact lives (the arena).
  // Getter-only was the bug: ES modules are strict, so an ordinary
  // `divWrapper.id = 'myCanvas'` *threw* and every MDN module example died
  // before rendering (bl-d313, found live on
  // mdn.github.io/js-examples/module-examples/).
  function reflectString(attr) {
    return {
      configurable: true,
      get: function () {
        return this.getAttribute(attr) || '';
      },
      set: function (v) {
        this.setAttribute(attr, String(v));
      },
    };
  }
  // The maker, shared with elem2.js's `type` over the same non-enumerable-global
  // seam brand.js uses — one rule for the whole family, not a descriptor per
  // attribute.
  Object.defineProperty(g, '__frot_reflect', {
    value: g.__frot_brand(reflectString, '__frot_reflect'),
    configurable: true,
    writable: true,
  });
  // `id` and `className`: assigned directly by every framework and by ordinary
  // page code, not only through setAttribute. `rel` (bl-07ab) is read off
  // MutationObserver records by the Vite modulepreload polyfill; routers read
  // `a.href` the same way.
  Object.defineProperty(proto, 'id', reflectString('id'));
  Object.defineProperty(proto, 'className', reflectString('class'));
  tagdef('Area Form Anchor Link', 'rel', reflectString('rel'));

  // classList lives in tokenlist.js: every token list the DOM exposes is a
  // spec-named DOMTokenList built by that one maker (bl-3a36).
  // `href` and `src` reflect RESOLVED against the document URL (both getters are
  // absolute in Firefox), falling back to the raw value when it will not parse.
  // One descriptor maker, because they are one rule: `src` earned its place when
  // Turbopack/Next chunks read `document.currentScript.src` to derive their own
  // chunk path (bl-a19d) — `new URL(that)` needs the absolute form.
  function reflectUrl(name) {
    return {
      configurable: true,
      get: function () {
        var v = this.getAttribute(name);
        if (v == null) return '';
        try {
          return new g.URL(v, g.location.href).href;
        } catch (e) {
          return v;
        }
      },
      set: function (v) {
        this.setAttribute(name, String(v));
      },
    };
  }
  tagdef('Area Base Anchor Link', 'href', reflectUrl('href'));
  tagdef('Image Media Track Script IFrame Embed Frame Input Source', 'src', reflectUrl('src'));

  // --- dataset: data-* reflection ------------------------------------------
  var dataset = {
    configurable: true,
    get: function () {
      var el = this;
      return new Proxy(Object.create(null), {
        get: function (_t, p) {
          if (typeof p !== 'string') return undefined;
          var v = el.getAttribute('data-' + dash(p));
          return v == null ? undefined : v;
        },
        set: function (_t, p, v) {
          el.setAttribute('data-' + dash(p), String(v));
          return true;
        },
        has: function (_t, p) {
          return typeof p === 'string' && el.hasAttribute('data-' + dash(p));
        },
      });
    },
  };
  STYLED.forEach(function (P) {
    Object.defineProperty(P, 'dataset', dataset);
  });
})(globalThis);
