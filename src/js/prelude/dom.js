// DOM facade — handle-based Node/Element and the document, over the syscalls.
// A wrapper is a thin object around an integer NodeId, which lives in brand.js's
// instance-state WeakMap rather than on the wrapper (bl-3bdc: a real element
// owns no properties at all); the arena is the one source of truth (js.md §2),
// so every read/write is a fresh syscall and wrappers hold no cached state.
(function (g) {
  'use strict';
  // A node wrapper's arena id, like every other instance's backing state, lives
  // in brand.js's one WeakMap: a real element owns no properties at all, and
  // `_id` was one every page could read off `Object.getOwnPropertyNames`
  // (bl-3bdc, identity.md §3.16).
  var slots = g.__frot_slots;

  function wrap(id) {
    return id === null || id === undefined ? null : new Node(id);
  }

  // --- DOM collections: one maker, two spec-named interfaces (bl-e5c3) -------
  // The invariant that dissolves the collection zoo: every collection a DOM
  // query returns is an instance of the interface the web spec names for it —
  // childNodes / querySelectorAll / getElementsByName yield a NodeList,
  // children / getElementsByTagName / getElementsByClassName an HTMLCollection
  // — and both are built by this one maker: a snapshot of wrapped nodes as own
  // indexed props over the interface prototype. Like every wrapper, a
  // collection holds no arena state beyond the nodes it was asked to hold.
  // WebIDL shapes the prototypes: `new NodeList()` throws Illegal constructor
  // (via __frot_iface), `length` is a prototype accessor, and the iteration
  // methods ARE the Array.prototype ones — exactly as Gecko exposes them.
  var A = Array.prototype;
  var hasOwn = Object.prototype.hasOwnProperty;
  function item(i) {
    var n = this[i >>> 0];
    return n === undefined ? null : n;
  }
  function collectionLength() {
    var n = 0;
    while (hasOwn.call(this, n)) n++;
    return n;
  }
  // Methods first, `length` after: that is the own-property order Gecko's
  // `NodeList.prototype`/`HTMLCollection.prototype` enumerate in (measured,
  // identity.md §3.15 — operations, then attributes, then `constructor`).
  function defineCollection(name, methods) {
    var Ctor = g.__frot_iface(name);
    Object.keys(methods).forEach(function (k) {
      Object.defineProperty(Ctor.prototype, k, {
        value: g.__frot_brand(methods[k], k),
        writable: true,
        enumerable: true,
        configurable: true,
      });
    });
    g.__frot_ifaceattrs(Ctor.prototype, { length: collectionLength });
    Object.defineProperty(Ctor.prototype, Symbol.iterator, {
      value: A.values,
      writable: true,
      configurable: true,
    });
    return Ctor;
  }
  var NodeList = defineCollection('NodeList', {
    item: item,
    keys: A.keys,
    values: A.values,
    entries: A.entries,
    forEach: A.forEach,
  });
  var HTMLCollection = defineCollection('HTMLCollection', {
    item: item,
    namedItem: function namedItem(name) {
      var s = String(name);
      for (var i = 0; hasOwn.call(this, i); i++) {
        var n = this[i];
        if (n.getAttribute('id') === s || n.getAttribute('name') === s) return n;
      }
      return null;
    },
  });
  function collect(Ctor, ids) {
    var c = Object.create(Ctor.prototype);
    for (var i = 0; i < ids.length; i++) {
      Object.defineProperty(c, i, { value: wrap(ids[i]), enumerable: true, configurable: true });
    }
    return c;
  }
  function wrapAll(ids) {
    return collect(NodeList, ids);
  }
  // The HTMLCollection maker, shared with elem2.js's getElementsBy* (the same
  // non-enumerable-global seam brand.js uses for __frot_brand/__frot_iface).
  Object.defineProperty(g, '__frot_elems', {
    value: g.__frot_brand(function (ids) {
      return collect(HTMLCollection, ids);
    }, '__frot_elems'),
    configurable: true,
    writable: true,
  });
  // The NodeList maker, shared with observer.js's MutationRecord
  // addedNodes/removedNodes (bl-07ab) — the same seam pattern as __frot_elems.
  Object.defineProperty(g, '__frot_nodelist', {
    value: g.__frot_brand(wrapAll, '__frot_nodelist'),
    configurable: true,
    writable: true,
  });

  class Node {
    constructor(id) {
      slots(this).id = id;
    }
    get nodeType() {
      var k = g.__frot_kind(slots(this).id);
      return k === 'element' ? 1 : k === 'text' ? 3 : k === 'comment' ? 8 : 10;
    }
    get nodeName() {
      return this.tagName || '#' + g.__frot_kind(slots(this).id);
    }
    get tagName() {
      var t = g.__frot_tag(slots(this).id);
      return t ? t.toUpperCase() : undefined;
    }
    get parentNode() {
      return wrap(g.__frot_parent(slots(this).id));
    }
    get childNodes() {
      return wrapAll(g.__frot_children(slots(this).id));
    }
    get children() {
      return g.__frot_elems(
        g.__frot_children(slots(this).id).filter(function (id) {
          return g.__frot_kind(id) === 'element';
        })
      );
    }
    get firstChild() {
      var kids = g.__frot_children(slots(this).id);
      return kids.length ? wrap(kids[0]) : null;
    }
    get textContent() {
      return g.__frot_text(slots(this).id);
    }
    set textContent(value) {
      clear(this);
      g.__frot_insert_child(slots(this).id, g.__frot_create_text(String(value)), null);
    }
    getAttribute(name) {
      // The arena stores attribute names ASCII-lowercased (the parser and
      // set_attr both normalize), so reads normalize too — SVG's camelCase
      // (viewBox) round-trips instead of silently missing (bl-3a36).
      var v = g.__frot_attr(slots(this).id, String(name).toLowerCase());
      return v === null || v === undefined ? null : v;
    }
    setAttribute(name, value) {
      g.__frot_set_attr(slots(this).id, name, String(value));
    }
    removeAttribute(name) {
      g.__frot_remove_attr(slots(this).id, name);
    }
    hasAttribute(name) {
      return this.getAttribute(name) !== null;
    }
    appendChild(child) {
      g.__frot_insert_child(slots(this).id, slots(child).id, null);
      return child;
    }
    insertBefore(child, ref) {
      // The reference sibling goes through as a node id: the arena resolves the
      // slot after the move unlinks `child`, so a same-parent move (or a ref
      // that is `child` itself) needs no index arithmetic here (bl-ae88).
      g.__frot_insert_child(slots(this).id, slots(child).id, ref ? slots(ref).id : null);
      return child;
    }
    removeChild(child) {
      g.__frot_detach(slots(child).id);
      return child;
    }
    // Replacement is one operation, and the DOM already names it (§4.2.3): the
    // reference is `child`'s NEXT sibling, taken *before* the removal. Naming a
    // sibling instead of an index (bl-ae88) is what makes replacing a node with
    // itself — which `document.body = document.body` is — the general path with
    // its own successor as the reference, not a special case.
    replaceChild(node, child) {
      var kids = g.__frot_children(slots(this).id);
      var at = kids.indexOf(slots(child).id);
      if (at < 0) {
        throw g.__frot_domerror(
          'NotFoundError',
          'Node.replaceChild: Child to be replaced is not a child of this node'
        );
      }
      var ref = at + 1 < kids.length ? kids[at + 1] : null;
      g.__frot_detach(slots(child).id);
      g.__frot_insert_child(slots(this).id, slots(node).id, ref);
      return child;
    }
    querySelector(sel) {
      var hits = g.__frot_query(slots(this).id, sel);
      return hits.length ? wrap(hits[0]) : null;
    }
    querySelectorAll(sel) {
      return wrapAll(g.__frot_query(slots(this).id, sel));
    }
  }

  // Detach every child of `node`. A module-local function published on the same
  // non-enumerable seam as `__frot_elems`, NOT a `_clear` method on
  // `Node.prototype`: a page reads that prototype's own-property list, and
  // Gecko's carries no such member (bl-3bdc).
  function clear(node) {
    var kids = g.__frot_children(slots(node).id);
    for (var i = 0; i < kids.length; i++) g.__frot_detach(kids[i]);
  }
  Object.defineProperty(g, '__frot_clear', {
    value: g.__frot_brand(clear, '__frot_clear'),
    configurable: true,
    writable: true,
  });

  var document = {
    get documentElement() {
      var roots = g.__frot_roots();
      var first = null;
      for (var i = 0; i < roots.length; i++) {
        if (g.__frot_kind(roots[i]) !== 'element') continue;
        if (first === null) first = roots[i];
        if (g.__frot_tag(roots[i]) === 'html') return wrap(roots[i]);
      }
      return wrap(first);
    },
    // "The body element" is the first body **or frameset** in the document
    // (HTML §3.1.5) — the two the setter below accepts, so the pair is one rule
    // read one way and written the other.
    get body() {
      return this.querySelector('body, frameset');
    },
    // Writable, as in a browser (HTML "the body element"): the assignment
    // REPLACES the current body — it is `replaceChild` under a type rule, not a
    // stored field, so the arena stays the one home for which element that is.
    // A value that is not a body/frameset throws HierarchyRequestError; getter-
    // only turned both the swap and the type error into silence (bl-273b).
    set body(el) {
      var tag = el && el.tagName;
      if (tag !== 'BODY' && tag !== 'FRAMESET') {
        throw g.__frot_domerror(
          'HierarchyRequestError',
          'Document.body: The new body must be either a body or frameset element'
        );
      }
      var old = this.body;
      if (old) old.parentNode.replaceChild(el, old);
      else this.documentElement.appendChild(el);
    },
    get head() {
      return this.querySelector('head');
    },
    createElement: function (tag) {
      return wrap(g.__frot_create_element(String(tag)));
    },
    // Namespaced creation (bl-3a36): React/Vue create every SVG/MathML element
    // through createElementNS — absent, any icon-bearing app dies in commit.
    // The arena stores local names only (the parser's foreign-content handling
    // does the same), so the namespace argument is honestly dropped.
    createElementNS: function (_ns, tag) {
      return wrap(g.__frot_create_element(String(tag)));
    },
    createTextNode: function (text) {
      return wrap(g.__frot_create_text(String(text)));
    },
    querySelector: function (sel) {
      var hits = g.__frot_query_doc(sel);
      return hits.length ? wrap(hits[0]) : null;
    },
    querySelectorAll: function (sel) {
      return wrapAll(g.__frot_query_doc(sel));
    },
    getElementById: function (id) {
      return this.querySelector('#' + id);
    },
    // The classic script executing right now (js.md §4.1), `null` between
    // scripts and during module evaluation. Turbopack/Next chunks derive their
    // own URL from it (bl-a19d), so its absence took every Next site dark. A
    // getter over the host's cell, not a stored field: the host owns the fact,
    // so a script that throws cannot strand a stale one here.
    get currentScript() {
      return wrap(g.__frot_current_script());
    },
  };

  g.Node = Node;
  g.document = document;
})(globalThis);
