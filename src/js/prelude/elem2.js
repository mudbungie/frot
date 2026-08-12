// Element/Node methods + document extensions the frameworks lean on (js.md §3),
// split from elem.js to stay under the 300-line source cap: fragment-aware
// insertion, collections (getElementsBy*/matches/contains), faithful cloneNode,
// lastChild + form-control reflections, and the `document` breadth jQuery's
// feature detection probes. Loads after elem.js (needs its Node breadth).
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

  // --- document extensions -------------------------------------------------
  g.document.createDocumentFragment = function () {
    return wrapFragment();
  };
  // `new DocumentFragment()` is real (bl-3a36): Radix-style libraries portal
  // into a fresh fragment (`createPortal(children, new DocumentFragment())`),
  // and React checks the container's nodeType — elem.js's hasInstance-only
  // stub constructed a plain object and the app died with React #299. The
  // constructor returns the same staging fragment createDocumentFragment
  // hands out; instanceof keeps matching by nodeType.
  var FragmentCtor = function DocumentFragment() {
    return wrapFragment();
  };
  Object.defineProperty(FragmentCtor, Symbol.hasInstance, {
    value: function (o) {
      return !!o && typeof o === 'object' && o.nodeType === 11;
    },
  });
  g.DocumentFragment = FragmentCtor;
  // A real arena comment (bl-79db): frameworks insert comments as anchors and
  // navigate from them (parentNode/nextSibling) to place later content — a
  // fake here strands every such patch (Vue RouterView/v-if), a silent dead app.
  g.document.createComment = function (text) {
    return new Node(g.__frot_create_comment(String(text)));
  };
  g.document.getElementsByTagName = function (tag) {
    return g.__frot_elems(g.__frot_query_doc(String(tag)));
  };
  g.document.getElementsByClassName = function (cls) {
    return g.__frot_elems(g.__frot_query_doc('.' + String(cls).trim().split(/\s+/).join('.')));
  };
  g.document.getElementsByName = function (name) {
    return g.document.querySelectorAll('[name="' + String(name) + '"]');
  };
  g.document.readyState = 'loading';
  g.document.nodeType = 9; // DOCUMENT_NODE — jQuery's setDocument gates on this.
  g.document.nodeName = '#document';
  g.document.ownerDocument = null;
  g.document.defaultView = g;
  g.document.compatMode = 'CSS1Compat';
  g.document.implementation = {
    // An *isolated* throwaway document — its body is a detached <body> so a
    // probe like jQuery's `createHTMLDocument("").body.innerHTML = ...` (a parse
    // feature-test) mutates a subtree the pipeline never walks, never the live
    // page. One arena (§2), so the detachment is the isolation.
    createHTMLDocument: function () {
      var body = g.document.createElement('body');
      var html = g.document.createElement('html');
      html.appendChild(body);
      return {
        body: body,
        documentElement: html,
        createElement: function (t) {
          return g.document.createElement(t);
        },
        createTextNode: function (t) {
          return g.document.createTextNode(t);
        },
      };
    },
    hasFeature: function () {
      return true;
    },
  };

  // A DocumentFragment: a detached staging parent. Children appended to it are
  // moved into the real target on the next appendChild/insertBefore (frameworks
  // batch DOM writes through fragments). In Firefox a fragment is a Node, hence
  // an EventTarget (bl-e81b): React listens on every portal container
  // (listenToAllSupportedEvents), and Radix-style libraries park closed-popover
  // content in `createPortal(children, new DocumentFragment())` — a missing
  // addEventListener throws mid-render, React's unwind misaligns its shared
  // cursor stack, and the *next* render dies on a corrupted context ("You
  // cannot render a <Router> inside another <Router>"). The trio is Node's own
  // (events.js — one registry, one implementation, loaded by the time a page can
  // construct a fragment), keyed per fragment by a unique negative node id no
  // arena node can ever carry.
  var fragSeq = 0;
  function wrapFragment() {
    var kids = [];
    function indexOfKid(n) {
      for (var i = 0; i < kids.length; i++) if (kids[i] === n || (n && slots(kids[i]).id === slots(n).id)) return i;
      return -1;
    }
    var frag = {
      addEventListener: proto.addEventListener,
      removeEventListener: proto.removeEventListener,
      dispatchEvent: proto.dispatchEvent,
      nodeType: 11,
      nodeName: '#document-fragment',
      // A portal container: React reaches the document through the container
      // (bl-3a36), so the staging fragment names the one document.
      ownerDocument: g.document,
      childNodes: kids,
      get firstChild() {
        return kids.length ? kids[0] : null;
      },
      appendChild: function (n) {
        kids.push(n);
        return n;
      },
      insertBefore: function (n, ref) {
        var i = ref ? indexOfKid(ref) : -1;
        if (i < 0) kids.push(n);
        else kids.splice(i, 0, n);
        return n;
      },
      removeChild: function (n) {
        var i = indexOfKid(n);
        if (i >= 0) kids.splice(i, 1);
        return n;
      },
    };
    // Node id and drain hook are STATE, not surface: `_id`/`_drain` were two own
    // names a page could read off the fragment (bl-3bdc).
    var st = slots(frag);
    st.id = --fragSeq;
    st.drain = function (target, at) {
      for (var i = 0; i < kids.length; i++) target.insertBefore(kids[i], at || null);
      kids.length = 0;
    };
    return frag;
  }
})(globalThis);
