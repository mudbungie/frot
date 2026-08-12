// The `document` object's own breadth and the DocumentFragment it hands out
// (js.md §3), split from elem2.js on the seam that file already named to stay
// under the 300-line source cap (bl-6da7): the document-level collections and
// the probe surface jQuery's feature detection reads (readyState, nodeType,
// compatMode, implementation.createHTMLDocument), real arena comments, and the
// detached staging fragment `createDocumentFragment()`/`new DocumentFragment()`
// return. Loads after elem2.js, whose appendChild/insertBefore drain a fragment
// through the `drain` slot set here.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var Node = g.Node;
  var proto = Node.prototype;

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
