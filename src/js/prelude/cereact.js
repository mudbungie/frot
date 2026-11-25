// Custom element reactions (js.md §15): the element queues, the upgrade
// algorithm, and the mutation seam they ride. The registry (ce.js) is this
// realm's — an iframe window runs its own copy of the prelude and gets its own
// — and its only arena write is an upgraded element's custom state, which
// `:defined` reads (`dom::live`).
//
// Reactions ride the mutation seam MutationObserver already wraps (js.md §7):
// creation, insertion, removal and attribute writes reach the arena through
// the syscalls below, so wrapping them is all it takes to see every one. A
// reaction is queued on its element and run when the outermost DOM operation
// returns — an element queue per [CEReactions] scope, as the HTML standard
// keeps them — which is what lets a cloned element's constructor see its
// children. A realm that defines nothing pays one test per mutation.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var brand = g.__frot_brand;
  var err = g.__frot_domerror;
  var raw = {};
  ['create_element', 'fragment', 'insert_child', 'detach', 'set_attr', 'remove_attr'].forEach(function (k) {
    raw[k] = g['__frot_' + k];
  });

  // The registry's state, shared with ce.js: definitions by name and by
  // constructor, the `is` values elements were made with, and the elements
  // being upgraded (innermost last).
  var ce = { defs: Object.create(null), ctors: new Map(), isValues: new Map(), building: [], count: 0 };
  var defs = ce.defs;
  var isValues = ce.isValues;

  var RESERVED = ' annotation-xml color-profile font-face font-face-src font-face-uri font-face-format ' +
    'font-face-name missing-glyph ';
  // HTML's valid custom element name: a lower-case ASCII letter, then
  // PCENChars with at least one hyphen, and not one of the eight reserved.
  var PCEN = /^[a-z][-.0-9_a-z\u00b7\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u037d\u037f-\u1fff\u200c\u200d\u203f\u2040\u2070-\u218f\u2c00-\u2fef\u3001-\ud7ff\uf900-\ufdcf\ufdf0-\ufffd\ud800-\udfff]*$/;
  function valid(name) {
    return PCEN.test(name) && name.indexOf('-') > 0 && RESERVED.indexOf(' ' + name + ' ') < 0;
  }

  // --- Element queues ([CEReactions]) ----------------------------------------
  var stack = [];
  function enqueue(w, reaction) {
    var st = slots(w);
    (st.reactions || (st.reactions = [])).push(reaction);
    var q = stack[stack.length - 1];
    if (!q.has.has(w)) {
      q.has.add(w);
      q.list.push(w);
    }
  }
  // Run `fn` as one [CEReactions] operation: reactions it queues run when it
  // returns, each element's in order, elements in the order first queued.
  function scope(fn, self, args) {
    var q = { list: [], has: new Set(), draining: false };
    stack.push(q);
    try {
      return fn.apply(self, args);
    } finally {
      q.draining = true;
      for (var i = 0; i < q.list.length; i++) {
        var st = slots(q.list[i]);
        while (st.reactions && st.reactions.length) react(q.list[i], st.reactions.shift());
      }
      stack.pop();
    }
  }
  // A syscall joins the operation around it, or is one by itself.
  function within(fn, self, args) {
    var q = stack[stack.length - 1];
    return q && !q.draining ? fn.apply(self, args) : scope(fn, self, args);
  }
  function react(w, r) {
    if (r === UPGRADE) return upgrade(w);
    var def = slots(w).custom;
    var cb = def && def.callbacks[r[0]];
    if (!cb) return;
    try {
      cb.apply(w, r[1]);
    } catch (e) {
      g.reportError(e);
    }
  }
  var UPGRADE = 'upgrade';

  // --- Definitions -----------------------------------------------------------
  // The definition an element of `local` with `is` value `is` takes: a valid
  // custom element name is its own definition's, whatever `is` says.
  function definition(local, is) {
    var def = defs[valid(local) ? local : is];
    return def && def.local === local ? def : null;
  }
  function lookup(w) {
    var id = slots(w).id;
    var is = isValues.has(id) ? isValues.get(id) : g.__frot_attr(id, 'is');
    return definition(g.__frot_tag(id), is);
  }
  // Upgrade an element (HTML "upgrade an element"): queue the callbacks its
  // present state owes, construct it in place, and mark it custom.
  function upgrade(w) {
    var st = slots(w);
    var def = !st.custom && !st.failed && lookup(w);
    if (!def) return;
    def.observed.forEach(function (name) {
      var v = g.__frot_attr(st.id, name);
      if (v !== null && v !== undefined) enqueue(w, ['attributeChangedCallback', [name, null, v, null]]);
    });
    if (g.__frot_connected(st.id)) enqueue(w, ['connectedCallback', []]);
    ce.building.push([def, w]);
    try {
      if (Reflect.construct(def.ctor, []) !== w) {
        throw err('NS_ERROR_UNEXPECTED', '');
      }
      st.custom = def;
      g.__frot_set_custom(st.id);
    } catch (e) {
      st.failed = true;
      st.reactions = [];
      g.reportError(e);
    } finally {
      ce.building.pop();
    }
  }
  // Every element a subtree holds that is, or could become, custom.
  function each(id, fn) {
    var ids = g.__frot_custom_nodes(id);
    for (var i = 0; i < ids.length; i++) fn(wrap(ids[i]));
  }
  function connect(w) {
    enqueue(w, slots(w).custom ? ['connectedCallback', []] : UPGRADE);
  }
  function disconnect(w) {
    if (slots(w).custom) enqueue(w, ['disconnectedCallback', []]);
  }

  // The `is` value an element was made with and carries in no attribute,
  // which markup.js serializes (js.md §15).
  Object.defineProperty(g, '__frot_is_value', {
    value: brand(function (id) {
      return isValues.get(id);
    }, '__frot_is_value'),
    configurable: true,
  });

  // --- The mutation seam -----------------------------------------------------
  function seam(name, around) {
    var f = raw[name];
    g['__frot_' + name] = brand(function () {
      if (!ce.count) return f.apply(this, arguments);
      return within(around, this, [f, arguments]);
    }, '__frot_' + name);
  }
  // A new element of a defined name is upgraded when the operation that made
  // it ends — after a clone has its children and attributes.
  seam('create_element', function (f, a) {
    var id = f.apply(null, a);
    enqueue(wrap(id), UPGRADE);
    return id;
  });
  seam('fragment', function (f, a) {
    var id = f.apply(null, a);
    each(id, function (w) {
      enqueue(w, UPGRADE);
    });
    return id;
  });
  seam('insert_child', function (f, a) {
    var child = a[1];
    var moved = g.__frot_kind(child) === 'document-fragment' ? g.__frot_children(child) : [child];
    var was = moved.map(function (id) {
      return g.__frot_connected(id);
    });
    var why = f.apply(null, a);
    if (why) return why;
    var now = g.__frot_connected(a[0]);
    moved.forEach(function (id, i) {
      if (was[i]) each(id, disconnect);
      if (now) each(id, connect);
    });
  });
  seam('detach', function (f, a) {
    var was = g.__frot_connected(a[0]);
    f.apply(null, a);
    if (was) each(a[0], disconnect);
  });
  function attr(f, a, value) {
    var w = wrap(a[0]);
    var def = slots(w).custom;
    var name = String(a[1]).toLowerCase();
    var old = def ? g.__frot_attr(a[0], name) : null;
    f.apply(null, a);
    if (def && def.observed.indexOf(name) >= 0) {
      enqueue(w, ['attributeChangedCallback', [name, old === undefined ? null : old, value, null]]);
    }
  }
  seam('set_attr', function (f, a) {
    attr(f, a, String(a[2]));
  });
  seam('remove_attr', function (f, a) {
    attr(f, a, null);
  });

  // The operations that are one [CEReactions] scope though they make several
  // syscalls: cloning (so a clone's constructor sees the whole clone) and the
  // markup setters (so the removed children's callbacks run first).
  function scoped(proto, key) {
    var d = Object.getOwnPropertyDescriptor(proto, key);
    ['value', 'set'].forEach(function (k) {
      var f = d[k];
      if (typeof f !== 'function') return;
      d[k] = brand(function () {
        return ce.count ? within(f, this, arguments) : f.apply(this, arguments);
      }, (k === 'set' ? 'set ' : '') + key);
    });
    Object.defineProperty(proto, key, d);
  }
  scoped(g.Node.prototype, 'cloneNode');
  scoped(g.Element.prototype, 'innerHTML');
  scoped(g.Element.prototype, 'outerHTML');
  scoped(g.ShadowRoot.prototype, 'innerHTML');

  ce.raw = raw;
  ce.valid = valid;
  ce.definition = definition;
  ce.lookup = lookup;
  ce.enqueue = enqueue;
  ce.scope = scope;
  ce.each = each;
  ce.UPGRADE = UPGRADE;
  Object.defineProperty(g, '__frot_ce', { value: ce, configurable: true });
})(globalThis);
