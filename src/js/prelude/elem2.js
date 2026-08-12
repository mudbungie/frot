// Node/Element prototype breadth the frameworks lean on (js.md §3), split from
// elem.js to stay under the 300-line source cap: fragment-aware insertion,
// collections (getElementsBy*/matches/contains), the ParentNode/ChildNode
// mixin, namespaced attributes, sibling reads, faithful cloneNode, lastChild
// and the form-control reflections. The `document` object's own breadth and the
// DocumentFragment it hands out are in doc.js, split on that seam by bl-6da7.
// Loads after elem.js (needs its Node breadth and its one reflection maker).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var Node = g.Node;
  var proto = Node.prototype;

  // --- fragment-aware insertion: a DocumentFragment (nodeType 11) drains its
  // batched children into the target; every other node a script can hold is a
  // real arena node (comments included, bl-79db), riding dom.js's inserts.
  var rawAppend = proto.appendChild;
  var rawInsert = proto.insertBefore;
  proto.appendChild = function (child) {
    if (child && child.nodeType === 11) return slots(child).drain(this, null), child;
    return rawAppend.call(this, child);
  };
  proto.insertBefore = function (child, ref) {
    if (child && child.nodeType === 11) return slots(child).drain(this, ref), child;
    return rawInsert.call(this, child, ref);
  };

  // --- collections + matches the frameworks lean on ------------------------
  // getElementsBy* return an HTMLCollection (the spec-named interface, built by
  // dom.js's one collection maker over the same selector hits qSA would find).
  proto.getElementsByTagName = function (tag) {
    return g.__frot_elems(g.__frot_query(slots(this).id, String(tag)));
  };
  proto.getElementsByClassName = function (cls) {
    return g.__frot_elems(g.__frot_query(slots(this).id, '.' + String(cls).trim().split(/\s+/).join('.')));
  };
  proto.matches = function (sel) {
    var hits = this.parentNode ? this.parentNode.querySelectorAll(sel) : g.document.querySelectorAll(sel);
    var len = hits.length;
    for (var i = 0; i < len; i++) if (slots(hits[i]).id === slots(this).id) return true;
    return false;
  };
  proto.contains = function (other) {
    for (var n = other; n; n = n.parentNode) if (slots(n).id === slots(this).id) return true;
    return false;
  };
  proto.closest = function (sel) {
    for (var n = this; n && n.nodeType === 1; n = n.parentNode) if (n.matches(sel)) return n;
    return null;
  };
  // The ParentNode/ChildNode convenience mixin (bl-3a36): append/prepend take
  // nodes or strings (a string becomes a text node, per spec); remove detaches
  // from the parent. All three ride the existing insertion/removal paths.
  function toNode(v) {
    return v && v.nodeType ? v : g.document.createTextNode(String(v));
  }
  proto.append = function () {
    for (var i = 0; i < arguments.length; i++) this.appendChild(toNode(arguments[i]));
  };
  proto.prepend = function () {
    var ref = this.firstChild;
    for (var i = 0; i < arguments.length; i++) this.insertBefore(toNode(arguments[i]), ref);
  };
  proto.remove = function () {
    var p = this.parentNode;
    if (p) p.removeChild(this);
  };
  // Namespaced attributes (bl-3a36): the arena stores plain attribute names
  // (the parser flattens foreign-content attrs the same way), so the NS
  // variants delegate — React sets xlink:href/xml:lang through setAttributeNS.
  proto.setAttributeNS = function (_ns, name, value) {
    this.setAttribute(name, value);
  };
  proto.getAttributeNS = function (_ns, name) {
    return this.getAttribute(name);
  };
  proto.removeAttributeNS = function (_ns, name) {
    this.removeAttribute(name);
  };
  // getRootNode (bl-3a36): no shadow DOM (js.md §11), so the root is the document for
  // connected nodes and the subtree top for detached ones — walk the one arena.
  proto.getRootNode = function () {
    var top = this;
    while (top.parentNode) top = top.parentNode;
    var roots = g.__frot_roots();
    for (var i = 0; i < roots.length; i++) if (roots[i] === slots(top).id) return g.document;
    return top;
  };
  // Sibling reads off the parent's live child list (no cached state, §2).
  function sibling(el, step) {
    var p = el.parentNode;
    if (!p) return null;
    var kids = p.childNodes;
    for (var i = 0; i < kids.length; i++)
      if (slots(kids[i]).id === slots(el).id) {
        var j = i + step;
        return j >= 0 && j < kids.length ? kids[j] : null;
      }
    return null;
  }
  Object.defineProperty(proto, 'nextSibling', {
    configurable: true,
    get: function () {
      return sibling(this, 1);
    },
  });
  Object.defineProperty(proto, 'previousSibling', {
    configurable: true,
    get: function () {
      return sibling(this, -1);
    },
  });
  // Faithful deep clone: element attributes (via __frot_attrs) are copied, then
  // children recursively — jQuery's support detection and its clone-based
  // fragment builder both depend on attribute/child fidelity a serialize round
  // trip would blur.
  proto.cloneNode = function (deep) {
    if (this.nodeType === 3) return g.document.createTextNode(this.textContent);
    if (this.nodeType !== 1) return g.document.createTextNode('');
    var copy = g.document.createElement(this.tagName);
    var attrs = g.__frot_attrs(slots(this).id);
    for (var i = 0; i < attrs.length; i++) copy.setAttribute(attrs[i][0], attrs[i][1]);
    if (deep) {
      var kids = this.childNodes;
      for (var j = 0; j < kids.length; j++) copy.appendChild(kids[j].cloneNode(true));
    }
    return copy;
  };

  // lastChild + the form-control reflections frameworks/jQuery read off cloned
  // nodes (checked/value/defaultValue/selected/disabled/type). Each mirrors an
  // attribute, so the arena stays the one source (no cached state).
  Object.defineProperty(proto, 'lastChild', {
    configurable: true,
    get: function () {
      var kids = g.__frot_children(slots(this).id);
      return kids.length ? new Node(kids[kids.length - 1]) : null;
    },
  });
  function reflectBool(name) {
    return {
      configurable: true,
      get: function () {
        return this.hasAttribute(name);
      },
      set: function (v) {
        if (v) this.setAttribute(name, name);
        else this.removeAttribute(name);
      },
    };
  }
  Object.defineProperty(proto, 'checked', reflectBool('checked'));
  Object.defineProperty(proto, 'selected', reflectBool('selected'));
  Object.defineProperty(proto, 'disabled', reflectBool('disabled'));
  Object.defineProperty(proto, 'value', {
    configurable: true,
    get: function () {
      return this.tagName === 'TEXTAREA' ? this.textContent : this.getAttribute('value') || '';
    },
    set: function (v) {
      this.setAttribute('value', String(v));
    },
  });
  Object.defineProperty(proto, 'defaultValue', {
    configurable: true,
    get: function () {
      return this.tagName === 'TEXTAREA' ? this.textContent : this.getAttribute('value') || '';
    },
    // React sets node.defaultValue when it mounts a controlled <input>; with no
    // setter that throws mid-render and the app dies. defaultValue reflects the
    // same `value` attribute the getter reads (frot caches no form state — the
    // arena attribute is the one source), so the setter mirrors `value`'s.
    set: function (v) {
      this.setAttribute('value', String(v));
    },
  });
  // `type` is a plain reflected string, so it comes from elem.js's one maker
  // (bl-d313) rather than a fourth hand-written descriptor.
  Object.defineProperty(proto, 'type', g.__frot_reflect('type'));
  // The form-control interface prototypes carry the same reflection
  // descriptors (bl-3a36): Radix-style libraries read
  // `Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set`
  // and call it — on an empty iface prototype that is `.set` of undefined and
  // the app dies in render. The getters/setters are the generic attribute
  // reflections above, so sharing the descriptor keeps one source of truth.
  [g.HTMLInputElement, g.HTMLTextAreaElement, g.HTMLSelectElement].forEach(function (C) {
    ['value', 'defaultValue', 'checked', 'selected', 'disabled', 'type'].forEach(function (name) {
      Object.defineProperty(C.prototype, name, Object.getOwnPropertyDescriptor(proto, name));
    });
  });
})(globalThis);
