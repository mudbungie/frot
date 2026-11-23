// TreeWalker and NodeFilter (bl-b8c0), DOM §6, every shape and message read
// off Firefox 153.0esr (identity.md §3.19 rig). A walker is a cursor over the
// one arena: it holds its root, whatToShow, filter and current node, and every
// step is a fresh read of the tree through the node syscalls — never through
// the public `firstChild`/`parentNode` accessors, which a page may replace.
// lit-html builds one at module load (`createTreeWalker(document, 129)`).
// Loads after doc.js.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var TYPE = { element: 1, text: 3, comment: 8, document: 9, doctype: 10, 'document-fragment': 11 };
  var ACCEPT = 1;
  var REJECT = 2;

  // NodeFilter is a callback interface: a function object with the constants
  // and NO `prototype`, which `new` and a call both refuse. A bound function is
  // the one constructor JS makes without a `prototype` property.
  var NodeFilter = g.__frot_brand(function () {
    throw new TypeError('Illegal constructor.');
  }.bind(), 'NodeFilter');
  [['FILTER_ACCEPT', 1], ['FILTER_REJECT', 2], ['FILTER_SKIP', 3], ['SHOW_ALL', 4294967295]]
    .concat(('ELEMENT ATTRIBUTE TEXT CDATA_SECTION ENTITY_REFERENCE ENTITY PROCESSING_INSTRUCTION ' +
      'COMMENT DOCUMENT DOCUMENT_TYPE DOCUMENT_FRAGMENT NOTATION').split(' ').map(function (n, i) {
      return ['SHOW_' + n, 1 << i];
    }))
    .forEach(function (c) {
      Object.defineProperty(NodeFilter, c[0], { value: c[1], enumerable: true });
    });
  Object.defineProperty(g, 'NodeFilter', { value: NodeFilter, configurable: true, writable: true });

  // The arena id a node wrapper stands for; anything else has none.
  function idOf(v) {
    var id = slots(v).id;
    return typeof id === 'number' ? id : undefined;
  }
  function parent(id) {
    var p = g.__frot_parent(id);
    return p === null || p === undefined ? null : p;
  }
  function child(id, last) {
    var ids = g.__frot_children(id);
    return ids.length ? ids[last ? ids.length - 1 : 0] : null;
  }
  function sibling(id, prev) {
    var p = parent(id);
    if (p === null) return null;
    var ids = g.__frot_children(p);
    var j = ids.indexOf(id) + (prev ? -1 : 1);
    return j >= 0 && j < ids.length ? ids[j] : null;
  }

  var TreeWalker = g.__frot_iface('TreeWalker');
  function state(w, member) {
    var st = slots(w);
    if (st.walker) return st;
    throw new TypeError("'" + member + "' called on an object that does not implement interface " +
      'TreeWalker.');
  }
  // DOM §6.1 "filter": whatToShow first, then the page's filter under the
  // active flag. A callable is called with `this` undefined; otherwise its
  // `acceptNode` with `this` the filter. The answer is a WebIDL unsigned short.
  function filter(st, id) {
    if (st.active) {
      throw g.__frot_domerror('InvalidStateError',
        'An attempt was made to use an object that is not, or is no longer, usable');
    }
    if (!(st.whatToShow & (1 << (TYPE[g.__frot_kind(id)] - 1)))) return 3;
    var f = st.filter;
    if (f === null) return ACCEPT;
    st.active = true;
    try {
      var fn = typeof f === 'function' ? f : f.acceptNode;
      if (typeof fn !== 'function') throw new TypeError("Property 'acceptNode' is not callable.");
      return (+fn.call(typeof f === 'function' ? undefined : f, wrap(id)) >>> 0) & 0xffff;
    } finally {
      st.active = false;
    }
  }
  function accept(st, id) {
    st.current = id;
    return wrap(id);
  }
  // DOM §6.2 "traverse children": `last` walks from the last child backwards.
  function children(st, last) {
    var node = child(st.current, last);
    while (node !== null) {
      var result = filter(st, node);
      if (result === ACCEPT) return accept(st, node);
      var kid = result === 3 ? child(node, last) : null;
      if (kid !== null) {
        node = kid;
        continue;
      }
      while (node !== null) {
        var sib = sibling(node, last);
        if (sib !== null) {
          node = sib;
          break;
        }
        var up = parent(node);
        if (up === null || up === st.root || up === st.current) return null;
        node = up;
      }
    }
    return null;
  }
  // DOM §6.2 "traverse siblings".
  function siblings(st, prev) {
    var node = st.current;
    if (node === st.root) return null;
    for (;;) {
      var sib = sibling(node, prev);
      while (sib !== null) {
        node = sib;
        var result = filter(st, node);
        if (result === ACCEPT) return accept(st, node);
        sib = result === REJECT ? null : child(node, prev);
        if (sib === null) sib = sibling(node, prev);
      }
      node = parent(node);
      if (node === null || node === st.root || filter(st, node) === ACCEPT) return null;
    }
  }

  g.__frot_ifaceops(TreeWalker.prototype, {
    parentNode: function parentNode() {
      var st = state(this, 'parentNode');
      var node = st.current;
      while (node !== null && node !== st.root) {
        node = parent(node);
        if (node !== null && filter(st, node) === ACCEPT) return accept(st, node);
      }
      return null;
    },
    firstChild: function firstChild() {
      return children(state(this, 'firstChild'), false);
    },
    lastChild: function lastChild() {
      return children(state(this, 'lastChild'), true);
    },
    previousSibling: function previousSibling() {
      return siblings(state(this, 'previousSibling'), true);
    },
    nextSibling: function nextSibling() {
      return siblings(state(this, 'nextSibling'), false);
    },
    previousNode: function previousNode() {
      var st = state(this, 'previousNode');
      var node = st.current;
      while (node !== st.root) {
        var sib = sibling(node, true);
        while (sib !== null) {
          node = sib;
          var result = filter(st, node);
          for (var kid; result !== REJECT && (kid = child(node, true)) !== null;) {
            node = kid;
            result = filter(st, node);
          }
          if (result === ACCEPT) return accept(st, node);
          sib = sibling(node, true);
        }
        var up = parent(node);
        if (node === st.root || up === null) return null;
        node = up;
        if (filter(st, node) === ACCEPT) return accept(st, node);
      }
      return null;
    },
    // A walk that leaves the root's subtree (a removed current node, or one
    // set outside the root) ends rather than climbing past the top.
    nextNode: function nextNode() {
      var st = state(this, 'nextNode');
      var node = st.current;
      var result = ACCEPT;
      for (;;) {
        for (var kid; result !== REJECT && (kid = child(node, false)) !== null;) {
          node = kid;
          result = filter(st, node);
          if (result === ACCEPT) return accept(st, node);
        }
        for (var tmp = node, sib = null; sib === null; tmp = parent(tmp)) {
          if (tmp === null || tmp === st.root) return null;
          sib = sibling(tmp, false);
        }
        node = sib;
        result = filter(st, node);
        if (result === ACCEPT) return accept(st, node);
      }
    },
  });
  g.__frot_ifaceattrs(TreeWalker.prototype, {
    root: function () {
      return wrap(state(this, 'get root').root);
    },
    whatToShow: function () {
      return state(this, 'get whatToShow').whatToShow;
    },
    filter: function () {
      return state(this, 'get filter').filter;
    },
  });
  Object.defineProperty(TreeWalker.prototype, 'currentNode', {
    get: g.__frot_brand(function () {
      return wrap(state(this, 'get currentNode').current);
    }, 'get currentNode'),
    set: g.__frot_brand(function (v) {
      var st = state(this, 'set currentNode');
      var at = 'TreeWalker.currentNode setter: Value being assigned ';
      if (v === null || (typeof v !== 'object' && typeof v !== 'function')) {
        throw new TypeError(at + 'is not an object.');
      }
      if (idOf(v) === undefined) throw new TypeError(at + 'does not implement interface Node.');
      st.current = idOf(v);
    }, 'set currentNode'),
    enumerable: true,
    configurable: true,
  });

  // Document.createTreeWalker(root, whatToShow = SHOW_ALL, filter = null).
  g.__frot_ifaceops(g.Document.prototype, {
    createTreeWalker: function createTreeWalker(root) {
      var at = 'Document.createTreeWalker: ';
      if (!arguments.length) throw new TypeError(at + 'At least 1 argument required, but only 0 passed');
      if (root === null || (typeof root !== 'object' && typeof root !== 'function')) {
        throw new TypeError(at + 'Argument 1 is not an object.');
      }
      if (idOf(root) === undefined) throw new TypeError(at + 'Argument 1 does not implement interface Node.');
      var f = arguments[2] === undefined ? null : arguments[2];
      if (f !== null && typeof f !== 'object' && typeof f !== 'function') {
        throw new TypeError(at + 'Argument 3 is not an object.');
      }
      var w = Object.create(TreeWalker.prototype);
      var st = slots(w);
      st.walker = true;
      st.root = st.current = idOf(root);
      st.whatToShow = arguments[1] === undefined ? 4294967295 : arguments[1] >>> 0;
      st.filter = f;
      st.active = false;
      return w;
    },
  });
})(globalThis);
