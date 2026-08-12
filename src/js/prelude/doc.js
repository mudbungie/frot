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
  var iface = g.__frot_iface;
  var Node = g.Node;

  // `new DocumentFragment()` is real (bl-3a36): Radix-style libraries portal
  // into a fresh fragment (`createPortal(children, new DocumentFragment())`),
  // and React checks the container's nodeType — elem.js's hasInstance-only stub
  // constructed a plain object and the app died with React #299. The constructor
  // and createDocumentFragment() build the same staging fragment.
  g.document.createDocumentFragment = function () {
    return new DocumentFragment();
  };
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

  // --- DocumentFragment ------------------------------------------------------
  // A detached staging parent. Children appended to it are moved into the real
  // target on the next appendChild/insertBefore (frameworks batch DOM writes
  // through fragments). In Firefox a fragment is a Node, hence an EventTarget
  // (bl-e81b): React listens on every portal container
  // (listenToAllSupportedEvents), and Radix-style libraries park closed-popover
  // content in `createPortal(children, new DocumentFragment())` — a missing
  // addEventListener throws mid-render, React's unwind misaligns its shared
  // cursor stack, and the *next* render dies on a corrupted context ("You
  // cannot render a <Router> inside another <Router>").
  //
  // It is a real interface now (bl-643d): it was an object literal owning all
  // eleven of its members, where a measured Firefox 153.0esr fragment owns
  // NOTHING (identity.md §3.17) and answers off `DocumentFragment.prototype`.
  //
  // NOT asserted, recorded instead: on the binary the parent is `Node.prototype`
  // and the prototype carries only getElementById, prepend, append,
  // replaceChildren, moveBefore, querySelector, querySelectorAll, children,
  // firstElementChild, lastElementChild, childElementCount — appendChild,
  // insertBefore, removeChild, childNodes and firstChild all come from Node.
  // frot's fragment is a STAGING object outside the arena (its children have not
  // been inserted anywhere yet), so those five cannot be the arena-backed Node
  // implementations and stay here; the parent is EventTarget, which is where the
  // listener trio genuinely comes from. Making the fragment a real arena node is
  // the only way to close that, and it is a host change, not a shape change.
  var fragSeq = 0;
  var DocumentFragment = iface('DocumentFragment', null, function (inst) {
    var st = slots(inst);
    st.kids = [];
    st.id = --fragSeq;
    st.drain = function (target, at) {
      var kids = st.kids;
      for (var i = 0; i < kids.length; i++) target.insertBefore(kids[i], at || null);
      kids.length = 0;
    };
  });
  Object.setPrototypeOf(DocumentFragment.prototype, g.EventTarget.prototype);
  function indexOfKid(kids, n) {
    for (var i = 0; i < kids.length; i++)
      if (kids[i] === n || (n && slots(kids[i]).id === slots(n).id)) return i;
    return -1;
  }
  g.__frot_ifaceops(DocumentFragment.prototype, {
    appendChild: function appendChild(n) {
      slots(this).kids.push(n);
      return n;
    },
    insertBefore: function insertBefore(n, ref) {
      var kids = slots(this).kids;
      var i = ref ? indexOfKid(kids, ref) : -1;
      if (i < 0) kids.push(n);
      else kids.splice(i, 0, n);
      return n;
    },
    removeChild: function removeChild(n) {
      var kids = slots(this).kids;
      var i = indexOfKid(kids, n);
      if (i >= 0) kids.splice(i, 1);
      return n;
    },
  });
  g.__frot_ifaceattrs(DocumentFragment.prototype, {
    nodeType: function () {
      return 11;
    },
    nodeName: function () {
      return '#document-fragment';
    },
    // A portal container: React reaches the document through the container
    // (bl-3a36), so the staging fragment names the one document.
    ownerDocument: function () {
      return g.document;
    },
    childNodes: function () {
      return slots(this).kids;
    },
    firstChild: function () {
      var kids = slots(this).kids;
      return kids.length ? kids[0] : null;
    },
  });
})(globalThis);
