// Element/Node property breadth the frameworks exercise (js.md §3): the
// `instanceof` interface constructors, the `style` facade, text `nodeValue`,
// `ownerDocument`, writable `className`, and `dataset`. Layered on
// dom.js's handle-based Node (no cached state — every read/write is a fresh
// syscall against the one arena, js.md §2). Loads after dom.js; the Node methods
// and document breadth are in elem2.js (kept split under the 300-line cap).
(function (g) {
  'use strict';
  var Node = g.Node;
  var proto = Node.prototype;

  // --- DOM interface constructors: frameworks feature-test with `instanceof`
  // (ReactDOM: `node instanceof HTMLElement`). Our nodes are all `Node`; each
  // interface is a constructor whose `Symbol.hasInstance` matches by nodeType/
  // tag, so `instanceof` answers without a class hierarchy. Absent interfaces
  // (iframe/input/...) simply never match — honest, never throwing.
  function iface(pred) {
    var f = function () {};
    Object.defineProperty(f, Symbol.hasInstance, {
      value: function (o) {
        return !!o && typeof o === 'object' && pred(o);
      },
    });
    return f;
  }
  var byType = function (t) {
    return function (o) {
      return o.nodeType === t;
    };
  };
  var byTag = function (tag) {
    return function (o) {
      return o.nodeType === 1 && o.tagName === tag;
    };
  };
  g.Element = iface(byType(1));
  g.HTMLElement = iface(byType(1));
  g.SVGElement = iface(byTag('SVG'));
  g.Text = iface(byType(3));
  g.Comment = iface(byType(8));
  g.DocumentFragment = iface(byType(11));
  g.HTMLIFrameElement = iface(byTag('IFRAME'));
  g.HTMLInputElement = iface(byTag('INPUT'));
  g.HTMLTextAreaElement = iface(byTag('TEXTAREA'));
  g.HTMLSelectElement = iface(byTag('SELECT'));

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
          t.cssText = String(v);
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
  Object.defineProperty(proto, 'style', {
    configurable: true,
    get: function () {
      return styleFor(this);
    },
  });

  // --- text nodeValue: element textContent already sets/reads; a text node's
  // value is the same arena text (frameworks patch text via `node.nodeValue`).
  Object.defineProperty(proto, 'nodeValue', {
    configurable: true,
    get: function () {
      return this.nodeType === 1 ? null : this.textContent;
    },
    set: function (v) {
      if (this.nodeType !== 1) g.__frot_set_text(this._id, String(v));
    },
  });
  Object.defineProperty(proto, 'data', {
    configurable: true,
    get: function () {
      return this.textContent;
    },
    set: function (v) {
      g.__frot_set_text(this._id, String(v));
    },
  });

  // ownerDocument: the one document (no frames, §11).
  Object.defineProperty(proto, 'ownerDocument', {
    configurable: true,
    get: function () {
      return g.document;
    },
  });

  // className is writable (frameworks assign it directly, not only setAttribute).
  Object.defineProperty(proto, 'className', {
    configurable: true,
    get: function () {
      return this.getAttribute('class') || '';
    },
    set: function (v) {
      this.setAttribute('class', String(v));
    },
  });

  // classList lives in tokenlist.js: every token list the DOM exposes is a
  // spec-named DOMTokenList built by that one maker (bl-3a36).

  // rel/href reflection (bl-07ab): MutationObserver's flagship field consumer —
  // the Vite modulepreload polyfill — reads `link.rel`/`link.href` off observed
  // records; routers read `a.href` the same way. `rel` reflects the attribute;
  // `href` reflects it RESOLVED against the document URL (Firefox's href getter
  // is absolute), falling back to the raw value when it will not parse.
  Object.defineProperty(proto, 'rel', {
    configurable: true,
    get: function () {
      return this.getAttribute('rel') || '';
    },
    set: function (v) {
      this.setAttribute('rel', String(v));
    },
  });
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
  Object.defineProperty(proto, 'href', reflectUrl('href'));
  Object.defineProperty(proto, 'src', reflectUrl('src'));

  // --- dataset: data-* reflection ------------------------------------------
  Object.defineProperty(proto, 'dataset', {
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
  });
})(globalThis);
