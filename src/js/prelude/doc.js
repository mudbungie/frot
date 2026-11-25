// The members of Document (bl-9f79, identity.md §3.21). `document` is the
// wrapper of the arena's document node — `HTMLDocument.prototype <-
// Document.prototype <- Node.prototype <- EventTarget.prototype`, as on the
// binary — so the tree operations and attributes are Node's, the ParentNode
// members come from elem2.js's mixin, and this file holds what is Document's
// own: creation, the document-level collections, and the probe surface
// jQuery's feature detection reads (readyState, compatMode,
// implementation.createHTMLDocument). The instance owns `location` alone
// (envobj.js). Loads after elem2.js.
(function (g) {
  'use strict';
  var wrap = g.__frot_wrap;
  var slots = g.__frot_slots;
  var P = g.Document.prototype;

  // What importNode and adoptNode refuse before they move anything.
  function foreign(op, argc, node) {
    var at = 'Document.' + op + ': ';
    if (!argc) throw new TypeError(at + 'At least 1 argument required, but only 0 passed');
    if (!(node instanceof g.Node)) throw new TypeError(at + 'Argument 1 does not implement interface Node.');
    if (node === g.document) throw g.__frot_domerror('NotSupportedError', 'Operation is not supported');
  }

  function query(doc, sel) {
    return g.__frot_query(slots(doc).id, sel);
  }

  g.__frot_ifaceops(P, {
    createElement: function createElement(tag) {
      return wrap(g.__frot_create_element(String(tag)));
    },
    // Namespaced creation (bl-3a36): React/Vue create every SVG/MathML element
    // through createElementNS — absent, any icon-bearing app dies in commit.
    // The arena stores local names only (the parser's foreign-content handling
    // does the same), so the namespace argument is honestly dropped.
    createElementNS: function createElementNS(_ns, tag) {
      return wrap(g.__frot_create_element(String(tag)));
    },
    // A fragment is an arena node (bl-de06): a parentless holder whose
    // children the arena moves when it is inserted.
    createDocumentFragment: function createDocumentFragment() {
      return wrap(g.__frot_create_fragment());
    },
    createTextNode: function createTextNode(text) {
      return wrap(g.__frot_create_text(String(text)));
    },
    // A real arena comment (bl-79db): frameworks insert comments as anchors and
    // navigate from them (parentNode/nextSibling) to place later content.
    createComment: function createComment(text) {
      return wrap(g.__frot_create_comment(String(text)));
    },
    // One document (js.md §2), so importing is cloning — the copy is already
    // this document's — and adopting is removing. The refusals are Firefox
    // 153.0esr's (js.md §15).
    importNode: function importNode(node) {
      foreign('importNode', arguments.length, node);
      return node.cloneNode(!!arguments[1]);
    },
    adoptNode: function adoptNode(node) {
      foreign('adoptNode', arguments.length, node);
      g.__frot_detach(slots(node).id);
      return node;
    },
    // An uninitialized event of the named interface (event.js's table).
    createEvent: function createEvent(name) {
      if (!arguments.length)
        throw new TypeError('Document.createEvent: At least 1 argument required, but only 0 passed');
      return g.__frot_create_event(name);
    },
    getElementsByName: function getElementsByName(name) {
      return this.querySelectorAll('[name="' + String(name) + '"]');
    },
  });

  // The stable objects a document hands out, made once.
  function once(key, make) {
    return function () {
      var st = slots(this);
      return st[key] || (st[key] = make());
    };
  }
  function child(doc, kind) {
    var kids = g.__frot_children(slots(doc).id).filter(function (id) {
      return g.__frot_kind(id) === kind;
    });
    return wrap(kids[0]);
  }
  g.__frot_ifaceattrs(P, {
    implementation: once('implementation', function () {
      return {
        // An *isolated* throwaway document — its body is a detached <body> so a
        // probe like jQuery's `createHTMLDocument("").body.innerHTML = ...` (a
        // parse feature-test) mutates a subtree the pipeline never walks, never
        // the live page. One arena (§2), so the detachment is the isolation.
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
    }),
    compatMode: function () {
      return 'CSS1Compat';
    },
    // The document's one doctype and one element child (the tree holds at
    // most one of each).
    doctype: function () {
      return child(this, 'doctype');
    },
    documentElement: function () {
      return child(this, 'element');
    },
    // Advanced by the host lifecycle (loop.js) through the slot.
    readyState: function () {
      return slots(this).readyState || 'loading';
    },
    head: function () {
      return this.querySelector('head');
    },
    // document.scripts (bl-c296): AWS WAF's challenge walks it looking for its
    // own integration script. On Firefox 153.0esr it is an HTMLCollection of
    // every <script> in the document, in tree order. Like every collection frot
    // hands out it is a snapshot taken at the read, where Firefox's is one live
    // object (`document.scripts === document.scripts`).
    scripts: function () {
      return g.__frot_elems(query(this, 'script'));
    },
    defaultView: function () {
      return g;
    },
    // The classic script executing right now (js.md §4.1), `null` between
    // scripts and during module evaluation. Turbopack/Next chunks derive their
    // own URL from it (bl-a19d). A getter over the host's cell, not a stored
    // field: the host owns the fact, so a script that throws cannot strand a
    // stale one here.
    currentScript: function () {
      return wrap(g.__frot_current_script());
    },
  });
  g.__frot_onevent(P, 'onreadystatechange');

  // "The body element" is the first body **or frameset** in the document (HTML
  // §3.1.5) — the two the setter accepts, so the pair is one rule read one way
  // and written the other. Writable, as in a browser: the assignment REPLACES
  // the current body — `replaceChild` under a type rule, not a stored field, so
  // the arena stays the one home for which element that is (bl-273b).
  Object.defineProperty(P, 'body', {
    get: function () {
      return this.querySelector('body, frameset');
    },
    set: function (el) {
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
    configurable: true,
  });
})(globalThis);
