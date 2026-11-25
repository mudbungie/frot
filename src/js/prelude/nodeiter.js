// NodeIterator (bl-33df), DOM §6.1, every shape and message read off Firefox
// 153.0esr (src/js/tests/nodeiter/). An iterator is a reference node and a
// pointer before or after it, over its root's inclusive descendants in tree
// order; each step re-reads the arena through the node syscalls and filters
// as a TreeWalker does (treewalker.js). The one piece of state a document
// keeps for them is the list of iterators it made: DOM's "NodeIterator
// pre-removing steps" move a reference node out of a subtree before that
// subtree leaves the tree, and every removal — `__frot_detach`, or an insert
// that moves a node out of its parent or a fragment — passes the seam below.
// The list lives and dies with the window, like MutationObserver's
// registrations (observer.js).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var filter = g.__frot_node_filter;
  var live = [];

  function parent(id) {
    var p = g.__frot_parent(id);
    return p === null || p === undefined ? null : p;
  }
  function kids(id) {
    return g.__frot_children(id);
  }
  // The node after `id` in tree order within `root`, skipping `id`'s own
  // subtree when `skip` is set.
  function following(id, root, skip) {
    var k = skip ? [] : kids(id);
    if (k.length) return k[0];
    for (var n = id; n !== null && n !== root; n = parent(n)) {
      var p = parent(n);
      if (p === null) return null;
      var sibs = kids(p), j = sibs.indexOf(n);
      if (j + 1 < sibs.length) return sibs[j + 1];
    }
    return null;
  }
  // The last inclusive descendant of `id` in tree order.
  function last(id) {
    for (var k = kids(id); k.length; k = kids(id)) id = k[k.length - 1];
    return id;
  }
  function preceding(id, root) {
    var p = id === root ? null : parent(id);
    if (p === null) return null;
    var sibs = kids(p), j = sibs.indexOf(id);
    return j > 0 ? last(sibs[j - 1]) : p;
  }
  function inclusiveAncestor(a, id) {
    for (var n = id; n !== null; n = parent(n)) if (n === a) return true;
    return false;
  }

  var NodeIterator = g.__frot_iface('NodeIterator');
  function state(it, member) {
    var st = slots(it);
    if (st.iterator) return st;
    throw new TypeError("'" + member + "' called on an object that does not implement interface " +
      'NodeIterator.');
  }
  // DOM §6.1 "traverse".
  function traverse(st, next) {
    var node = st.reference, before = st.before;
    for (;;) {
      if (next && !before) node = following(node, st.root, false);
      else if (!next && before) node = preceding(node, st.root);
      if (node === null) return null;
      before = !next;
      if (filter(st, node) === 1) break;
    }
    st.reference = node;
    st.before = before;
    return wrap(node);
  }
  g.__frot_ifaceops(NodeIterator.prototype, {
    nextNode: function nextNode() {
      return traverse(state(this, 'nextNode'), true);
    },
    previousNode: function previousNode() {
      return traverse(state(this, 'previousNode'), false);
    },
    detach: function detach() {
      state(this, 'detach');
    },
  });
  g.__frot_ifaceattrs(NodeIterator.prototype, {
    root: function () {
      return wrap(state(this, 'get root').root);
    },
    referenceNode: function () {
      return wrap(state(this, 'get referenceNode').reference);
    },
    pointerBeforeReferenceNode: function () {
      return state(this, 'get pointerBeforeReferenceNode').before;
    },
    whatToShow: function () {
      return state(this, 'get whatToShow').whatToShow;
    },
    filter: function () {
      return state(this, 'get filter').filter;
    },
  });

  // DOM §6.1 "NodeIterator pre-removing steps", for every live iterator.
  function preRemove(gone) {
    live.forEach(function (st) {
      if (gone === st.root || !inclusiveAncestor(gone, st.reference)) return;
      if (st.before) {
        var next = following(gone, st.root, true);
        if (next !== null) {
          st.reference = next;
          return;
        }
        st.before = false;
      }
      var p = parent(gone), sibs = kids(p), j = sibs.indexOf(gone);
      st.reference = j > 0 ? last(sibs[j - 1]) : p;
    });
  }
  var rawDetach = g.__frot_detach, rawInsert = g.__frot_insert_child;
  g.__frot_detach = g.__frot_brand(function (id) {
    if (live.length && parent(id) !== null) preRemove(id);
    return rawDetach(id);
  }, '__frot_detach');
  // An insert moves its node (a fragment's children) out first; a refused one
  // moved nothing, so the iterators are put back as they were.
  g.__frot_insert_child = g.__frot_brand(function (p, child, before) {
    var saved = live.map(function (st) {
      return [st.reference, st.before];
    });
    var moving = g.__frot_kind(child) === 'document-fragment' ? kids(child) : [child];
    moving.forEach(function (id) {
      if (live.length && parent(id) !== null) preRemove(id);
    });
    var why = rawInsert(p, child, before);
    if (why) {
      live.forEach(function (st, i) {
        st.reference = saved[i][0];
        st.before = saved[i][1];
      });
    }
    return why;
  }, '__frot_insert_child');

  // Document.createNodeIterator(root, whatToShow = SHOW_ALL, filter = null).
  g.__frot_ifaceops(g.Document.prototype, {
    createNodeIterator: function createNodeIterator(root) {
      var at = 'Document.createNodeIterator: ';
      if (!arguments.length) throw new TypeError(at + 'At least 1 argument required, but only 0 passed');
      if (root === null || (typeof root !== 'object' && typeof root !== 'function')) {
        throw new TypeError(at + 'Argument 1 is not an object.');
      }
      var id = slots(root).id;
      if (typeof id !== 'number') throw new TypeError(at + 'Argument 1 does not implement interface Node.');
      var f = arguments[2] === undefined ? null : arguments[2];
      if (f !== null && typeof f !== 'object' && typeof f !== 'function') {
        throw new TypeError(at + 'Argument 3 is not an object.');
      }
      var it = Object.create(NodeIterator.prototype);
      var st = slots(it);
      st.iterator = true;
      st.root = st.reference = id;
      st.before = true;
      st.whatToShow = arguments[1] === undefined ? 4294967295 : arguments[1] >>> 0;
      st.filter = f;
      st.active = false;
      live.push(st);
      return it;
    },
  });
})(globalThis);
