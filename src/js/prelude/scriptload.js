// What a <script> element adds (js.md §4, bl-a86b): `text`, the rest of
// HTMLScriptElement's reflected members and `HTMLScriptElement.supports`
// (bl-f9f2, read off Firefox 153.0esr), and the `load` or `error` the host
// fires at an external one.
//
// An external <script>'s `load` or `error`. When the host
// has run an external script — or failed to fetch it — it fires one of the two
// at the element, as Firefox does for every external script, parsed or
// inserted: `load` after it ran (even if it threw), `error` when its fetch
// failed. A handler's throw is reported like any listener's (events.js).
(function (g) {
  'use strict';
  // `text` is the script's source: its text, and setting it replaces the
  // children with one text node — which can make an inserted, empty script
  // runnable, exactly as `textContent` does.
  g.__frot_tagdef('Script', 'text', {
    get: function () {
      return this.textContent;
    },
    set: function (v) {
      this.textContent = String(v);
    },
  });
  // `async` is HTML's "force async" flag or the attribute: a script a script
  // makes is async until something sets `async` (an arena flag, js.md §4.7).
  // The rest reflect their attributes (enumerated ones as the binary
  // normalises them: measured).
  function attr(name, read, write) {
    return {
      get: function () {
        return read(this.getAttribute(name));
      },
      set: function (v) {
        if (write) return write(this, v);
        this.setAttribute(name, String(v));
      },
    };
  }
  function text(v) {
    return v || '';
  }
  function flag(name) {
    return attr(name, function (v) {
      return v !== null;
    }, function (el, v) {
      if (v) el.setAttribute(name, '');
      else el.removeAttribute(name);
    });
  }
  function keyword(words, missing, bad) {
    return function (v) {
      if (v === null) return missing;
      var k = v.toLowerCase();
      return words.indexOf(k) >= 0 ? k : bad;
    };
  }
  var id = function (el) {
    return g.__frot_slots(el).id;
  };
  var members = {
    noModule: flag('nomodule'),
    charset: attr('charset', text),
    async: {
      get: function () {
        return __frot_force_async(id(this)) || this.hasAttribute('async');
      },
      set: function (v) {
        __frot_unforce(id(this));
        flag('async').set.call(this, v);
      },
    },
    defer: flag('defer'),
    crossOrigin: attr('crossorigin', keyword(['anonymous', 'use-credentials'], null, 'anonymous'),
      function (el, v) {
        if (v === null) el.removeAttribute('crossorigin');
        else el.setAttribute('crossorigin', String(v));
      }),
    referrerPolicy: attr('referrerpolicy', keyword(['no-referrer', 'no-referrer-when-downgrade',
      'same-origin', 'origin', 'strict-origin', 'origin-when-cross-origin',
      'strict-origin-when-cross-origin', 'unsafe-url'], '', '')),
    fetchPriority: attr('fetchpriority', keyword(['high', 'low', 'auto'], 'auto', 'auto')),
    event: attr('event', text),
    htmlFor: attr('for', text),
    integrity: attr('integrity', text),
  };
  Object.keys(members).forEach(function (k) {
    var d = members[k];
    g.__frot_brand(d.get, 'get ' + k);
    g.__frot_brand(d.set, 'set ' + k);
    g.__frot_tagdef('Script', k, d);
  });
  // The script types this host runs: classic and module. Firefox also says
  // `importmap`; frot resolves no import map (js.md §4.1), so it says no.
  g.__frot_ifaceops(g.HTMLScriptElement, {
    supports: function supports(type) {
      if (!arguments.length) {
        throw new TypeError('HTMLScriptElement.supports: At least 1 argument required, but only '
          + '0 passed');
      }
      type = String(type);
      return type === 'classic' || type === 'module';
    },
  });
  Object.defineProperty(g, '__frot_script_event', {
    configurable: true,
    value: function (id, type) {
      var el = g.__frot_wrap(id);
      var ev = new g.Event(type);
      g.__frot_slots(ev).trusted = true; // the browser raised it, not the page
      g.__frot_dispatch(el, ev); // its `on<type>` handler included
      return 0;
    },
  });
})(globalThis);
