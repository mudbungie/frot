// DOM facade — handle-based Node/Element and the document, over the syscalls.
// A wrapper is a thin object around an integer NodeId (`_id`); the arena is the
// one source of truth (js.md §2), so every read/write is a fresh syscall and
// wrappers hold no cached state.
(function (g) {
  'use strict';

  function wrap(id) {
    return id === null || id === undefined ? null : new Node(id);
  }
  function wrapAll(ids) {
    return ids.map(wrap);
  }

  class Node {
    constructor(id) {
      this._id = id;
    }
    get nodeType() {
      var k = g.__frot_kind(this._id);
      return k === 'element' ? 1 : k === 'text' ? 3 : k === 'comment' ? 8 : 10;
    }
    get nodeName() {
      return this.tagName || '#' + g.__frot_kind(this._id);
    }
    get tagName() {
      var t = g.__frot_tag(this._id);
      return t ? t.toUpperCase() : undefined;
    }
    get parentNode() {
      return wrap(g.__frot_parent(this._id));
    }
    get childNodes() {
      return wrapAll(g.__frot_children(this._id));
    }
    get children() {
      return this.childNodes.filter(function (n) {
        return n.nodeType === 1;
      });
    }
    get firstChild() {
      var kids = g.__frot_children(this._id);
      return kids.length ? wrap(kids[0]) : null;
    }
    get textContent() {
      return g.__frot_text(this._id);
    }
    set textContent(value) {
      this._clear();
      g.__frot_insert_child(this._id, g.__frot_create_text(String(value)), 0);
    }
    get innerHTML() {
      return this.childNodes
        .map(function (n) {
          return n.nodeType === 1 ? n.outerHTML : n.textContent;
        })
        .join('');
    }
    set innerHTML(html) {
      this._clear();
      var ids = g.__frot_fragment(String(html));
      for (var i = 0; i < ids.length; i++) {
        g.__frot_insert_child(this._id, ids[i], i);
      }
    }
    get outerHTML() {
      var tag = g.__frot_tag(this._id);
      if (!tag) return this.textContent;
      return '<' + tag + '>' + this.innerHTML + '</' + tag + '>';
    }
    get id() {
      return this.getAttribute('id') || '';
    }
    get className() {
      return this.getAttribute('class') || '';
    }
    getAttribute(name) {
      var v = g.__frot_attr(this._id, name);
      return v === null || v === undefined ? null : v;
    }
    setAttribute(name, value) {
      g.__frot_set_attr(this._id, name, String(value));
    }
    removeAttribute(name) {
      g.__frot_remove_attr(this._id, name);
    }
    hasAttribute(name) {
      return this.getAttribute(name) !== null;
    }
    appendChild(child) {
      g.__frot_insert_child(this._id, child._id, g.__frot_children(this._id).length);
      return child;
    }
    insertBefore(child, ref) {
      var kids = g.__frot_children(this._id);
      var at = ref ? kids.indexOf(ref._id) : kids.length;
      g.__frot_insert_child(this._id, child._id, at < 0 ? kids.length : at);
      return child;
    }
    removeChild(child) {
      g.__frot_detach(child._id);
      return child;
    }
    querySelector(sel) {
      var hits = g.__frot_query(this._id, sel);
      return hits.length ? wrap(hits[0]) : null;
    }
    querySelectorAll(sel) {
      return wrapAll(g.__frot_query(this._id, sel));
    }
    _clear() {
      var kids = g.__frot_children(this._id);
      for (var i = 0; i < kids.length; i++) g.__frot_detach(kids[i]);
    }
  }

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
    get body() {
      return this.querySelector('body');
    },
    get head() {
      return this.querySelector('head');
    },
    createElement: function (tag) {
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
  };

  g.Node = Node;
  g.document = document;
})(globalThis);
