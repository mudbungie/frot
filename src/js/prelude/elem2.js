// Element/Node methods + document extensions the frameworks lean on (js.md §3),
// split from elem.js to stay under the 300-line source cap: fragment-aware
// insertion, collections (getElementsBy*/matches/contains), faithful cloneNode,
// lastChild + form-control reflections, and the `document` breadth jQuery's
// feature detection probes. Loads after elem.js (needs its Node breadth).
(function (g) {
  'use strict';
  var Node = g.Node;
  var proto = Node.prototype;

  // --- fragment-aware insertion: a DocumentFragment (nodeType 11) drains its
  // batched children into the target; a non-arena node (comment/fake, _id < 0)
  // is a no-op so it never reaches a syscall. Wraps dom.js's arena inserts.
  var rawAppend = proto.appendChild;
  var rawInsert = proto.insertBefore;
  proto.appendChild = function (child) {
    if (child && child.nodeType === 11) return child._drain(this, null), child;
    if (child && child._id < 0) return child;
    return rawAppend.call(this, child);
  };
  proto.insertBefore = function (child, ref) {
    if (child && child.nodeType === 11) return child._drain(this, ref), child;
    if (child && child._id < 0) return child;
    return rawInsert.call(this, child, ref);
  };

  // --- collections + matches the frameworks lean on ------------------------
  proto.getElementsByTagName = function (tag) {
    return this.querySelectorAll(String(tag));
  };
  proto.getElementsByClassName = function (cls) {
    return this.querySelectorAll('.' + String(cls).trim().split(/\s+/).join('.'));
  };
  proto.matches = function (sel) {
    var hits = this.parentNode ? this.parentNode.querySelectorAll(sel) : g.document.querySelectorAll(sel);
    return hits.some(
      function (n) {
        return n._id === this._id;
      }.bind(this)
    );
  };
  proto.contains = function (other) {
    for (var n = other; n; n = n.parentNode) if (n._id === this._id) return true;
    return false;
  };
  // Faithful deep clone: element attributes (via __frot_attrs) are copied, then
  // children recursively — jQuery's support detection and its clone-based
  // fragment builder both depend on attribute/child fidelity a serialize round
  // trip would blur.
  proto.cloneNode = function (deep) {
    if (this.nodeType === 3) return g.document.createTextNode(this.textContent);
    if (this.nodeType !== 1) return g.document.createTextNode('');
    var copy = g.document.createElement(this.tagName);
    var attrs = g.__frot_attrs(this._id);
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
      var kids = g.__frot_children(this._id);
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
  Object.defineProperty(proto, 'type', {
    configurable: true,
    get: function () {
      return this.getAttribute('type') || '';
    },
    set: function (v) {
      this.setAttribute('type', String(v));
    },
  });

  // --- document extensions -------------------------------------------------
  g.document.createDocumentFragment = function () {
    return wrapFragment();
  };
  g.document.createComment = function (text) {
    return { nodeType: 8, textContent: String(text), _id: -1 };
  };
  g.document.getElementsByTagName = function (tag) {
    return g.document.querySelectorAll(String(tag));
  };
  g.document.getElementsByClassName = function (cls) {
    return g.document.querySelectorAll('.' + String(cls).trim().split(/\s+/).join('.'));
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
  // batch DOM writes through fragments).
  function wrapFragment() {
    var kids = [];
    return {
      _id: -1,
      nodeType: 11,
      childNodes: kids,
      get firstChild() {
        return kids.length ? kids[0] : null;
      },
      appendChild: function (n) {
        kids.push(n);
        return n;
      },
      _drain: function (target, at) {
        for (var i = 0; i < kids.length; i++) target.insertBefore(kids[i], at || null);
        kids.length = 0;
      },
    };
  }
})(globalThis);
