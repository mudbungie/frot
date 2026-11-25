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

  g.__frot_ifaceops(P, {
    getElementById: function getElementById(id) {
      return g.DocumentFragment.prototype.getElementById.call(this, id);
    },
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
  });
  reflect(g.HTMLSlotElement.prototype, 'name', 'name');
})(globalThis);
