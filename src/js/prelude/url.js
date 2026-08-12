// URL + URLSearchParams (js.md §7) — a WHATWG subset routers reach for
// (`new URL(location.href)`, `url.searchParams`). Parsing is delegated to the
// Rust `url` crate via __frot_url_parse, so URL semantics live in ONE place;
// there is no second URL parser in JS. URLSearchParams is built in JS over the
// parsed `search` string. `new URL` THROWS a TypeError on an invalid URL
// (unlike `location`, a fact that returns empties). No syscall dependency —
// standalone module.
(function (g) {
  'use strict';
  var attrs = g.__frot_ifaceattrs;
  // Parsed components and the pair list live in brand.js's one instance-state
  // WeakMap. A real Gecko `URL`/`URLSearchParams` owns no properties at all —
  // every component is a prototype accessor — and the `_`-prefixed slots these
  // replace were readable straight out of `Object.getOwnPropertyNames`
  // (bl-3bdc, identity.md §3.16). The two internal operations moved with them:
  // `_setQuery`/`_changed` were own members of the PROTOTYPES, equally visible.
  var slots = g.__frot_slots;

  var COMPONENTS = ['href', 'protocol', 'host', 'hostname', 'port', 'pathname', 'search', 'hash', 'origin'];

  // application/x-www-form-urlencoded: '+' is a space, the rest percent-decodes.
  function dec(s) {
    try {
      return decodeURIComponent(s.replace(/\+/g, ' '));
    } catch (e) {
      return s;
    }
  }
  function enc(s) {
    return encodeURIComponent(s).replace(/%20/g, '+');
  }

  function parsePairs(search) {
    var q = search.charAt(0) === '?' ? search.slice(1) : search;
    if (q === '') return [];
    return q
      .split('&')
      .filter(function (part) {
        return part !== '';
      })
      .map(function (part) {
        var i = part.indexOf('=');
        return i < 0 ? [dec(part), ''] : [dec(part.slice(0, i)), dec(part.slice(i + 1))];
      });
  }

  function URLSearchParams(init) {
    var st = slots(this);
    if (init instanceof URLSearchParams) st.pairs = pairs(init).slice();
    else if (typeof init === 'string') st.pairs = parsePairs(init);
    else if (Array.isArray(init))
      st.pairs = init.map(function (p) {
        return [String(p[0]), String(p[1])];
      });
    else if (init && typeof init === 'object')
      st.pairs = Object.keys(init).map(function (k) {
        return [k, String(init[k])];
      });
    else st.pairs = [];
    st.url = null; // set when owned by a URL, to reflect edits back into it
  }
  function pairs(sp) {
    return slots(sp).pairs;
  }
  // A mutation re-serializes the pairs into the owning URL's query (single
  // source of truth: the URL owns the query string; this is its editable view).
  function changed(sp) {
    if (slots(sp).url) setQuery(slots(sp).url, sp.toString());
  }
  URLSearchParams.prototype.get = function (name) {
    name = String(name);
    var ps = pairs(this);
    for (var i = 0; i < ps.length; i++) if (ps[i][0] === name) return ps[i][1];
    return null;
  };
  URLSearchParams.prototype.getAll = function (name) {
    name = String(name);
    return pairs(this)
      .filter(function (p) {
        return p[0] === name;
      })
      .map(function (p) {
        return p[1];
      });
  };
  URLSearchParams.prototype.has = function (name) {
    return this.get(String(name)) !== null;
  };
  URLSearchParams.prototype.append = function (name, value) {
    pairs(this).push([String(name), String(value)]);
    changed(this);
  };
  URLSearchParams.prototype.set = function (name, value) {
    name = String(name);
    value = String(value);
    var done = false;
    var st = slots(this);
    st.pairs = st.pairs.filter(function (p) {
      if (p[0] !== name) return true;
      if (done) return false;
      done = true;
      p[1] = value;
      return true;
    });
    if (!done) st.pairs.push([name, value]);
    changed(this);
  };
  URLSearchParams.prototype['delete'] = function (name) {
    name = String(name);
    slots(this).pairs = pairs(this).filter(function (p) {
      return p[0] !== name;
    });
    changed(this);
  };
  URLSearchParams.prototype.forEach = function (fn, thisArg) {
    pairs(this).forEach(function (p) {
      fn.call(thisArg, p[1], p[0], this);
    }, this);
  };
  URLSearchParams.prototype.keys = function () {
    return pairs(this)
      .map(function (p) {
        return p[0];
      })
      [Symbol.iterator]();
  };
  URLSearchParams.prototype.values = function () {
    return pairs(this)
      .map(function (p) {
        return p[1];
      })
      [Symbol.iterator]();
  };
  URLSearchParams.prototype.entries = function () {
    return pairs(this)
      .map(function (p) {
        return [p[0], p[1]];
      })
      [Symbol.iterator]();
  };
  URLSearchParams.prototype[Symbol.iterator] = URLSearchParams.prototype.entries;
  URLSearchParams.prototype.toString = function () {
    return pairs(this)
      .map(function (p) {
        return enc(p[0]) + '=' + enc(p[1]);
      })
      .join('&');
  };

  function URL(spec, base) {
    var b = base === undefined || base === null ? undefined : String(base);
    var r = g.__frot_url_parse(String(spec), b);
    if (!r.valid) throw new TypeError('Invalid URL: ' + spec);
    var st = slots(this);
    COMPONENTS.forEach(function (k) {
      st[k] = r[k];
    });
    st.sp = null;
  }
  attrs(URL.prototype, COMPONENTS);
  // Splice a new query into the authoritative href (string surgery, not a
  // re-parse): everything before `?`/`#`, the new query, then the fragment.
  function setQuery(url, query) {
    var st = slots(url);
    st.search = query ? '?' + query : '';
    var hashAt = st.href.indexOf('#');
    var frag = hashAt < 0 ? st.hash : st.href.slice(hashAt);
    var head = hashAt < 0 ? st.href : st.href.slice(0, hashAt);
    var qAt = head.indexOf('?');
    if (qAt >= 0) head = head.slice(0, qAt);
    st.href = head + st.search + frag;
  }
  attrs(URL.prototype, {
    searchParams: function () {
      var st = slots(this);
      if (!st.sp) {
        st.sp = new URLSearchParams(st.search);
        slots(st.sp).url = this;
      }
      return st.sp;
    },
  });
  URL.prototype.toString = function () {
    return slots(this).href;
  };
  URL.prototype.toJSON = function () {
    return slots(this).href;
  };

  g.URL = URL;
  g.URLSearchParams = URLSearchParams;
})(globalThis);
