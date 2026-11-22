// The `document` object's own breadth and the DocumentFragment it hands out
// (js.md §3), split from elem2.js on the seam that file already named to stay
// under the 300-line source cap (bl-6da7): the document-level collections and
// the probe surface jQuery's feature detection reads (readyState, nodeType,
// compatMode, implementation.createHTMLDocument), real arena comments and
// fragments. Loads after elem2.js.
(function (g) {
  'use strict';
  var wrap = g.__frot_wrap;
  var DOC = g.__frot_document;

  // The members dom.js's literal used to carry, moved here on the file's own
  // subject (bl-de06, to keep dom.js under the cap).
  var core = {
    get documentElement() {
      var roots = g.__frot_children(DOC);
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
      var hits = g.__frot_query(DOC, sel);
      return hits.length ? wrap(hits[0]) : null;
    },
    querySelectorAll: function (sel) {
      return g.__frot_nodelist(g.__frot_query(DOC, sel));
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
  Object.defineProperties(g.document, Object.getOwnPropertyDescriptors(core));


  // A fragment is an arena node (bl-de06): a parentless holder whose children
  // the arena moves when it is inserted. `new DocumentFragment()` (dom.js)
  // makes the same thing — Radix-style libraries portal into a fresh one
  // (`createPortal(children, new DocumentFragment())`), and React listens on
  // every portal container, which a fragment answers as the Node it is.
  g.document.createDocumentFragment = function () {
    return g.__frot_wrap(g.__frot_create_fragment());
  };
  // A real arena comment (bl-79db): frameworks insert comments as anchors and
  // navigate from them (parentNode/nextSibling) to place later content — a
  // fake here strands every such patch (Vue RouterView/v-if), a silent dead app.
  g.document.createComment = function (text) {
    return g.__frot_wrap(g.__frot_create_comment(String(text)));
  };
  g.document.getElementsByTagName = function (tag) {
    return g.__frot_elems(g.__frot_query(DOC, String(tag)));
  };
  g.document.getElementsByClassName = function (cls) {
    return g.__frot_elems(g.__frot_query(DOC, '.' + String(cls).trim().split(/\s+/).join('.')));
  };
  // document.scripts (bl-c296): AWS WAF's challenge walks it looking for its own
  // integration script, and threw on `undefined.length` (challenge.md §11.4). On
  // Firefox 153.0esr it is an HTMLCollection of every <script> in the document,
  // in tree order — ones a script inserted included, a detached one not. Like
  // every collection frot hands out it is a snapshot taken at the read, where
  // Firefox's is one live object (`document.scripts === document.scripts`).
  Object.defineProperty(g.document, 'scripts', {
    get: function () {
      return g.__frot_elems(g.__frot_query(DOC, 'script'));
    },
    enumerable: true,
    configurable: true,
  });
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
})(globalThis);
