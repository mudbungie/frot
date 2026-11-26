// Shadow roots and slots (js.md §15), over the arena's: a root is an arena
// node (`dom::shadow`) that `attachShadow` makes or the parser made from a
// `<template shadowrootmode>`, and slot assignment is a query the arena
// answers, never a list kept here. Shapes and refusals are Firefox
// 153.0esr's (js/tests/shadow/).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var brand = g.__frot_brand;
  var err = g.__frot_domerror;
  var at = 'Element.attachShadow: ';

  var ShadowRoot = g.__frot_extends(g.__frot_iface('ShadowRoot'), g.DocumentFragment);
  var P = ShadowRoot.prototype;
  // [host, closed, declarative, clonable, serializable, delegatesFocus, manual]
  function info(root) {
    return g.__frot_shadow_info(slots(root).id);
  }
  function flag(i) {
    return function () {
      return !!info(this)[i];
    };
  }

  // setHTMLUnsafe (js.md §15.10): `innerHTML`'s parse, admitting declarative
  // shadow roots.
  function unsafe(iface) {
    return function setHTMLUnsafe(html) {
      if (!arguments.length) {
        throw new TypeError(iface + '.setHTMLUnsafe: At least 1 argument required, but only 0 passed');
      }
      g.__frot_set_inner(this, html, (iface === 'ShadowRoot' ? this.host : this).localName, true);
    };
  }
  g.__frot_ifaceops(P, {
    getElementById: function getElementById(id) {
      return g.DocumentFragment.prototype.getElementById.call(this, id);
    },
    setHTMLUnsafe: unsafe('ShadowRoot'),
    getHTML: function getHTML() {
      return g.__frot_markup(slots(this).id, arguments[0] || {});
    },
  });
  g.__frot_ifaceattrs(P, {
    mode: function () {
      return info(this)[1] ? 'closed' : 'open';
    },
    delegatesFocus: flag(5),
    slotAssignment: function () {
      return info(this)[6] ? 'manual' : 'named';
    },
    clonable: flag(3),
    serializable: flag(4),
    host: function () {
      return wrap(info(this)[0]);
    },
  });
  g.__frot_onevent(P, 'onslotchange');
  Object.defineProperty(P, 'innerHTML', {
    get: brand(function () {
      return g.__frot_markup(slots(this).id);
    }, 'get innerHTML'),
    set: brand(function (html) {
      g.__frot_clear(this);
      // The host is the context element (HTML, ShadowRoot `innerHTML`).
      var host = this.host.localName;
      g.__frot_insert_child(slots(this).id, g.__frot_fragment(String(html), host), null);
    }, 'set innerHTML'),
    enumerable: true,
    configurable: true,
  });
  g.__frot_ifaceattrs(P, {
    activeElement: function () {
      var f = g.__frot_focused();
      return f !== null && f !== undefined && g.__frot_tree_root(f) === slots(this).id ? wrap(f) : null;
    },
  });

  // --- Element: attachShadow, shadowRoot, assignedSlot, slot, getHTML --------
  var E = g.Element.prototype;
  function choice(v, name, keys, iface) {
    var s = String(v);
    if (keys.indexOf(s) < 0) {
      throw new TypeError(at + "'" + s + "' (value of '" + name + "' member of ShadowRootInit) is not a valid " +
        'value for enumeration ' + iface + '.');
    }
    return s;
  }
  g.__frot_ifaceops(E, {
    attachShadow: function attachShadow(init) {
      if (!arguments.length) throw new TypeError(at + 'At least 1 argument required, but only 0 passed');
      var o = init == null ? {} : Object(init);
      // A dictionary's members are read in name order (WebIDL).
      var clonable = !!o.clonable;
      var focus = !!o.delegatesFocus;
      if (o.mode === undefined) throw new TypeError(at + "Missing required 'mode' member of ShadowRootInit.");
      var closed = choice(o.mode, 'mode', ['open', 'closed'], 'ShadowRootMode') === 'closed';
      var serializable = !!o.serializable;
      var manual = o.slotAssignment !== undefined &&
        choice(o.slotAssignment, 'slotAssignment', ['named', 'manual'], 'SlotAssignmentMode') === 'manual';
      var id = slots(this).id;
      var why = g.__frot_attach_shadow(id, closed, clonable, serializable, focus, manual);
      if (why) throw err('NotSupportedError', at + why);
      return wrap(g.__frot_shadow(id));
    },
    setHTMLUnsafe: unsafe('Element'),
    getHTML: function getHTML() {
      return g.__frot_markup(slots(this).id, arguments[0] || {});
    },
  });
  function assignedSlot() {
    return wrap(g.__frot_assigned_slot(slots(this).id));
  }
  g.__frot_ifaceattrs(E, {
    shadowRoot: function () {
      var root = g.__frot_shadow(slots(this).id);
      return root === null || root === undefined || g.__frot_shadow_info(root)[1] ? null : wrap(root);
    },
    assignedSlot: assignedSlot,
  });
  function reflect(proto, key, attr) {
    var d = g.__frot_reflect(attr);
    d.enumerable = true;
    Object.defineProperty(proto, key, d);
  }
  reflect(E, 'slot', 'slot');
  g.__frot_ifaceattrs(g.Text.prototype, { assignedSlot: assignedSlot });

  // --- HTMLSlotElement -------------------------------------------------------
  // HTML "find flattened slottables": a slot assigned into another slot is
  // replaced by what it holds; with nothing assigned, its own slottable
  // children (fallback) stand in.
  function inShadow(id) {
    return !!g.__frot_shadow_info(g.__frot_tree_root(id));
  }
  function flattened(id) {
    var ids = g.__frot_assigned(id);
    if (!ids.length) {
      ids = g.__frot_children(id).filter(function (c) {
        var k = g.__frot_kind(c);
        return k === 'element' || k === 'text';
      });
    }
    var out = [];
    ids.forEach(function (n) {
      if (g.__frot_tag(n) === 'slot' && inShadow(n)) out.push.apply(out, flattened(n));
      else out.push(n);
    });
    return out;
  }
  function assigned(slot, options) {
    var id = slots(slot).id;
    var ids = options && options.flatten ? (inShadow(id) ? flattened(id) : []) : g.__frot_assigned(id);
    return ids.map(wrap);
  }
  g.__frot_ifaceops(g.HTMLSlotElement.prototype, {
    assignedNodes: function assignedNodes() {
      return assigned(this, arguments[0]);
    },
    assignedElements: function assignedElements() {
      return assigned(this, arguments[0]).filter(function (n) {
        return n.nodeType === 1;
      });
    },
    // Manual assignment (js.md §15.9): the arena keeps each slot's list and
    // answers the slots whose list changed, which are signalled.
    assign: function assign() {
      var ids = Array.prototype.map.call(arguments, function (n, i) {
        if (!(n instanceof g.Element || n instanceof g.Text)) {
          throw new TypeError('HTMLSlotElement.assign: Argument ' + (i + 1) +
            ' could not be converted to any of: Element, Text.');
        }
        return slots(n).id;
      });
      g.__frot_assign(slots(this).id, ids).forEach(function (s) {
        g.__frot_signal_slot(s);
      });
    },
  });
  reflect(g.HTMLSlotElement.prototype, 'name', 'name');

  // --- slotchange (js.md §15.8) ----------------------------------------------
  // Assignment is a query, so a change is found by asking it around each
  // mutation: the slots of the shadow trees the mutation can touch
  // (`__frot_slot_scope`, one call), before and after. A slot that lost nodes is signalled before one that only gained
  // them, each group in tree order; an unassigned slot whose own children
  // change is signalled too (Firefox 153.0esr, js/tests/slotchange/).
  function snapshot(roots, also) {
    var m = new Map();
    roots.forEach(function (r) {
      g.__frot_query(r, 'slot').forEach(function (s) {
        m.set(s, g.__frot_assigned(s).join());
      });
    });
    also.forEach(function (s) {
      if (!m.has(s)) m.set(s, g.__frot_assigned(s).join());
    });
    return m;
  }
  function around(roots, fallback, raw, args) {
    var before = roots.length ? snapshot(roots, []) : null;
    var why = raw.f(args[0], args[1], args[2]);
    if (fallback.length && !why) fallback.forEach(g.__frot_signal_slot);
    if (!before || why) return why;
    var after = snapshot(roots, Array.from(before.keys()));
    var lost = [];
    var gained = [];
    after.forEach(function (now, s) {
      var was = before.has(s) ? before.get(s) : '';
      if (was !== now) (was ? lost : gained).push(s);
    });
    lost.concat(gained).forEach(g.__frot_signal_slot);
    return why;
  }
  // The syscall is called as a method of a realm object: called bare, a
  // frame's inserted script ran in the window whose script made the call.
  function seam(name, roots) {
    var raw = { f: g['__frot_' + name] };
    g['__frot_' + name] = brand(function () {
      var a = arguments;
      var r = roots(a);
      return around(r[0], r[1], raw, a);
    }, '__frot_' + name);
  }
  // Firefox parses markup into place: a slot the markup parser made with
  // fallback children is signalled when it reaches a shadow tree.
  var parsed = new Set();
  var parse = { f: g.__frot_fragment };
  g.__frot_fragment = brand(function (html, context, roots) {
    var f = parse.f(html, context, roots);
    parsed.add(f);
    return f;
  }, '__frot_fragment');
  function fallback(s) {
    return g.__frot_children(s).some(function (c) {
      var k = g.__frot_kind(c);
      return k === 'element' || k === 'text';
    });
  }
  seam('insert_child', function (a) {
    var r = g.__frot_slot_scope(a[0], a[1]);
    if (parsed.delete(a[1]) && r[0].length) r[1] = r[1].concat(g.__frot_query(a[1], 'slot').filter(fallback));
    return r;
  });
  seam('detach', function (a) {
    return g.__frot_slot_scope(null, a[0]);
  });
  // A `slot` attribute moves its element between its parent host's slots; a
  // slot's `name` moves nodes between its tree's slots.
  function attr(a) {
    var name = String(a[1]).toLowerCase();
    var p = name === 'slot' ? g.__frot_parent(a[0]) : null;
    var slot = name === 'name' && g.__frot_tag(a[0]) === 'slot';
    var root = p != null ? g.__frot_shadow(p) : slot ? g.__frot_tree_root(a[0]) : null;
    return [root != null && g.__frot_shadow_info(root) ? [root] : [], []];
  }
  seam('set_attr', attr);
  seam('remove_attr', attr);
})(globalThis);
