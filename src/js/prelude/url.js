// URL + URLSearchParams (js.md §7) — a WHATWG subset routers reach for
// (`new URL(location.href)`, `url.searchParams`). Parsing is delegated to the
// Rust `url` crate via __frot_url_parse, so URL semantics live in ONE place;
// there is no second URL parser in JS. URLSearchParams is built in JS over the
// parsed `search` string. `new URL` THROWS a TypeError on an invalid URL
// (unlike `location`, a fact that returns empties). No syscall dependency —
// standalone module.
(function (g) {
  'use strict';

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
    if (init instanceof URLSearchParams) this._pairs = init._pairs.slice();
    else if (typeof init === 'string') this._pairs = parsePairs(init);
    else if (Array.isArray(init))
      this._pairs = init.map(function (p) {
        return [String(p[0]), String(p[1])];
      });
    else if (init && typeof init === 'object')
      this._pairs = Object.keys(init).map(function (k) {
        return [k, String(init[k])];
      });
    else this._pairs = [];
    this._url = null; // set when owned by a URL, to reflect edits back into it
  }
  // A mutation re-serializes the pairs into the owning URL's query (single
  // source of truth: the URL owns the query string; this is its editable view).
  URLSearchParams.prototype._changed = function () {
    if (this._url) this._url._setQuery(this.toString());
  };
  URLSearchParams.prototype.get = function (name) {
    name = String(name);
    for (var i = 0; i < this._pairs.length; i++) if (this._pairs[i][0] === name) return this._pairs[i][1];
    return null;
  };
  URLSearchParams.prototype.getAll = function (name) {
    name = String(name);
    return this._pairs
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
    this._pairs.push([String(name), String(value)]);
    this._changed();
  };
  URLSearchParams.prototype.set = function (name, value) {
    name = String(name);
    value = String(value);
    var done = false;
    this._pairs = this._pairs.filter(function (p) {
      if (p[0] !== name) return true;
      if (done) return false;
      done = true;
      p[1] = value;
      return true;
    });
    if (!done) this._pairs.push([name, value]);
    this._changed();
  };
  URLSearchParams.prototype['delete'] = function (name) {
    name = String(name);
    this._pairs = this._pairs.filter(function (p) {
      return p[0] !== name;
    });
    this._changed();
  };
  URLSearchParams.prototype.forEach = function (fn, thisArg) {
    this._pairs.forEach(function (p) {
      fn.call(thisArg, p[1], p[0], this);
    }, this);
  };
  URLSearchParams.prototype.keys = function () {
    return this._pairs
      .map(function (p) {
        return p[0];
      })
      [Symbol.iterator]();
  };
  URLSearchParams.prototype.values = function () {
    return this._pairs
      .map(function (p) {
        return p[1];
      })
      [Symbol.iterator]();
  };
  URLSearchParams.prototype.entries = function () {
    return this._pairs
      .map(function (p) {
        return [p[0], p[1]];
      })
      [Symbol.iterator]();
  };
  URLSearchParams.prototype[Symbol.iterator] = URLSearchParams.prototype.entries;
  URLSearchParams.prototype.toString = function () {
    return this._pairs
      .map(function (p) {
        return enc(p[0]) + '=' + enc(p[1]);
      })
      .join('&');
  };

  function URL(spec, base) {
    var b = base === undefined || base === null ? undefined : String(base);
    var r = g.__frot_url_parse(String(spec), b);
    if (!r.valid) throw new TypeError('Invalid URL: ' + spec);
    var self = this;
    COMPONENTS.forEach(function (k) {
      self['_' + k] = r[k];
    });
    this._sp = null;
  }
  COMPONENTS.forEach(function (k) {
    Object.defineProperty(URL.prototype, k, {
      enumerable: true,
      get: function () {
        return this['_' + k];
      },
    });
  });
  // Splice a new query into the authoritative href (string surgery, not a
  // re-parse): everything before `?`/`#`, the new query, then the fragment.
  URL.prototype._setQuery = function (query) {
    this._search = query ? '?' + query : '';
    var hashAt = this._href.indexOf('#');
    var frag = hashAt < 0 ? this._hash : this._href.slice(hashAt);
    var head = hashAt < 0 ? this._href : this._href.slice(0, hashAt);
    var qAt = head.indexOf('?');
    if (qAt >= 0) head = head.slice(0, qAt);
    this._href = head + this._search + frag;
  };
  Object.defineProperty(URL.prototype, 'searchParams', {
    get: function () {
      if (!this._sp) {
        this._sp = new URLSearchParams(this._search);
        this._sp._url = this;
      }
      return this._sp;
    },
  });
  URL.prototype.toString = function () {
    return this._href;
  };
  URL.prototype.toJSON = function () {
    return this._href;
  };

  g.URL = URL;
  g.URLSearchParams = URLSearchParams;
})(globalThis);
