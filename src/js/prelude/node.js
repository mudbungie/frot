// The members of Node and CharacterData (bl-78cb), each on the interface
// Firefox 153.0esr puts it on and in the order its prototype lists them
// (identity.md §3.21): operations, attributes, the node-type and
// document-position constants, `constructor`. Node.prototype holds the tree
// operations and tree attributes and NO element member, so a text node, a
// comment and (bl-9f79) the document inherit exactly these. Every read and
// write is a fresh syscall against the one arena (js.md §2). Loads after
// dom.js, which builds the interfaces and the wrapper these return.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var Node = g.Node;
  var TYPE = { element: 1, attribute: 2, text: 3, comment: 8, document: 9, doctype: 10, 'document-fragment': 11 };

  // An Attr (attr.js) is a Node with no arena id: its state names the
  // attribute it stands for.
  function kindOf(n) {
    return slots(n).attr ? 'attribute' : g.__frot_kind(slots(n).id);
  }
  function kids(n) {
    return g.__frot_children(slots(n).id);
  }
  function at(n, pick) {
    var ids = kids(n);
    return ids.length ? wrap(ids[pick(ids.length)]) : null;
  }
  // Sibling reads off the parent's live child list (no cached state, js.md §2).
  function sibling(n, step) {
    var p = g.__frot_parent(slots(n).id);
    if (p === null || p === undefined) return null;
    var ids = g.__frot_children(p);
    var j = ids.indexOf(slots(n).id) + step;
    return j >= 0 && j < ids.length ? wrap(ids[j]) : null;
  }
  // An insertion the tree cannot hold: the arena says why, in Firefox's words
  // (src/dom/hierarchy.rs), and nothing has moved.
  function refuse(op, why) {
    if (why) throw g.__frot_domerror('HierarchyRequestError', 'Node.' + op + ': ' + why);
  }
  // Every node a script can hold is an arena node — a fragment included, whose
  // children the arena moves in its place — and the reference sibling goes
  // through as a node id, never an index (bl-ae88).
  // A node of another window's document is adopted first (adopt.js).
  function insert(op, parent, child, ref) {
    child = g.__frot_own(child);
    refuse(op, g.__frot_insert_child(slots(parent).id, slots(child).id, ref ? slots(ref).id : null));
    return child;
  }

  g.__frot_ifaceops(Node.prototype, {
    // The top of the node's tree — the document, a shadow root, or a
    // detached subtree's top — and `{composed: true}` goes on through each
    // shadow root to its host (js.md §15).
    getRootNode: function getRootNode(options) {
      var top = g.__frot_tree_root(slots(this).id);
      while (options && options.composed && g.__frot_shadow_info(top)) {
        top = g.__frot_tree_root(g.__frot_shadow_info(top)[0]);
      }
      return wrap(top);
    },
    hasChildNodes: function hasChildNodes() {
      return kids(this).length > 0;
    },
    insertBefore: function insertBefore(child, ref) {
      return insert('insertBefore', this, child, ref);
    },
    appendChild: function appendChild(child) {
      return insert('appendChild', this, child, null);
    },
    // Replacement is one operation (DOM §4.2.3): the reference is `child`'s
    // NEXT sibling, taken before the removal, so replacing a node with itself
    // is the general path with its own successor as the reference (bl-ae88).
    replaceChild: function replaceChild(node, child) {
      var ids = kids(this);
      var i = ids.indexOf(slots(child).id);
      if (i < 0) {
        throw g.__frot_domerror(
          'NotFoundError',
          'Node.replaceChild: Child to be replaced is not a child of this node'
        );
      }
      // Judged with `child` set aside, before anything is removed.
      node = g.__frot_own(node);
      refuse('replaceChild', g.__frot_replace_refusal(slots(this).id, slots(node).id, ids[i]));
      g.__frot_detach(ids[i]);
      insert('replaceChild', this, node, i + 1 < ids.length ? wrap(ids[i + 1]) : null);
      return child;
    },
    removeChild: function removeChild(child) {
      g.__frot_detach(slots(child).id);
      return child;
    },
    // Faithful clone: a node of the same kind, an element's attributes, and,
    // when deep, its children — jQuery's support detection and its clone-based
    // fragment builder depend on fidelity a serialize round trip would blur.
    cloneNode: function cloneNode(deep) {
      var id = slots(this).id;
      var kind = g.__frot_kind(id);
      var shadow = g.__frot_shadow(id);
      if (g.__frot_shadow_info(id)) throw g.__frot_domerror('NotSupportedError', 'Operation is not supported');
      var copy = wrap(
        kind === 'element' ? g.__frot_create_element(g.__frot_tag(id))
          : kind === 'document-fragment' ? g.__frot_create_fragment()
            : kind === 'comment' ? g.__frot_create_comment(g.__frot_text(id))
              : kind === 'doctype' ? g.__frot_copy_node(id)
                : g.__frot_create_text(kind === 'text' ? g.__frot_text(id) : '')
      );
      g.__frot_attrs(id).forEach(function (a) {
        g.__frot_set_attr(slots(copy).id, a[0], a[1]);
      });
      // A clone of a script that started is started too: it never runs.
      g.__frot_clone_started(id, slots(copy).id);
      if (deep) {
        kids(this).forEach(function (kid) {
          copy.appendChild(wrap(kid).cloneNode(true));
        });
        // A template's contents are cloned into the copy's (js.md §15).
        if (copy instanceof g.HTMLTemplateElement) copy.content.appendChild(this.content.cloneNode(true));
      }
      // A clonable shadow root is cloned with its host, deep whatever `deep`.
      var root = shadow !== null && shadow !== undefined && wrap(shadow);
      if (root && root.clonable) {
        var into = copy.attachShadow({ mode: root.mode, clonable: true, serializable: root.serializable,
          delegatesFocus: root.delegatesFocus, slotAssignment: root.slotAssignment });
        kids(root).forEach(function (kid) {
          into.appendChild(wrap(kid).cloneNode(true));
        });
      }
      return copy;
    },
    isSameNode: function isSameNode(other) {
      return other === this;
    },
    contains: function contains(other) {
      for (var n = other; n; n = n.parentNode) if (n === this) return true;
      return false;
    },
  });

  g.__frot_ifaceattrs(Node.prototype, {
    nodeType: function () {
      return TYPE[kindOf(this)];
    },
    // An element's upper-cased tag, a doctype's own name (`html`, measured),
    // else `#text`, `#comment`, `#document`, `#document-fragment`.
    nodeName: function () {
      if (slots(this).attr) return slots(this).attr.name;
      var id = slots(this).id;
      var tag = g.__frot_tag(id);
      var dt = g.__frot_doctype(id);
      return tag ? tag.toUpperCase() : dt ? dt[0] : '#' + kindOf(this);
    },
    isConnected: function () {
      return g.__frot_connected(slots(this).id);
    },
    // The one document (no frames, js.md §11), which has no owner itself.
    ownerDocument: function () {
      return this === g.document ? null : g.document;
    },
    parentNode: function () {
      return wrap(g.__frot_parent(slots(this).id));
    },
    parentElement: function () {
      var p = this.parentNode;
      return p && p.nodeType === 1 ? p : null;
    },
    childNodes: function () {
      return g.__frot_nodelist(kids(this));
    },
    firstChild: function () {
      return at(this, function () {
        return 0;
      });
    },
    lastChild: function () {
      return at(this, function (n) {
        return n - 1;
      });
    },
    previousSibling: function () {
      return sibling(this, -1);
    },
    nextSibling: function () {
      return sibling(this, 1);
    },
  });

  // Character data IS its arena text, an attribute's its value; an element or
  // doctype has no node value.
  function data(n) {
    return slots(n).attr ? n.value : g.__frot_text(slots(n).id);
  }
  // `null` is the empty string ([LegacyNullToEmptyString] on `data`; `nodeValue`
  // is nullable), measured on 153.0esr for both.
  function setData(v) {
    var text = v === null ? '' : String(v);
    if (slots(this).attr) this.value = text;
    else g.__frot_set_text(slots(this).id, text);
  }
  function rw(proto, key, get, set) {
    Object.defineProperty(proto, key, {
      get: g.__frot_brand(get, 'get ' + key),
      set: g.__frot_brand(set, 'set ' + key),
      enumerable: true,
      configurable: true,
    });
  }
  rw(
    Node.prototype,
    'nodeValue',
    function () {
      return this instanceof g.CharacterData || slots(this).attr ? data(this) : null;
    },
    function (v) {
      if (this instanceof g.CharacterData || slots(this).attr) setData.call(this, v);
    }
  );
  // Setting an element's textContent replaces its children with one text node,
  // or with none for the empty string; `null` is the empty string
  // ([LegacyNullToEmptyString]). A document and a doctype have none: the read
  // is `null` and the write does nothing (measured).
  var TEXTLESS = { document: true, doctype: true };
  rw(
    Node.prototype,
    'textContent',
    function () {
      return TEXTLESS[kindOf(this)] ? null : data(this);
    },
    function (v) {
      var text = v === null || v === undefined ? '' : String(v);
      if (TEXTLESS[kindOf(this)]) return;
      if (this instanceof g.CharacterData || slots(this).attr) return setData.call(this, text);
      g.__frot_clear(this);
      if (text) g.__frot_insert_child(slots(this).id, g.__frot_create_text(text), null);
    }
  );

  // The constants sit on the interface object AND its prototype, read-only and
  // non-configurable, as measured.
  ('ELEMENT_NODE ATTRIBUTE_NODE TEXT_NODE CDATA_SECTION_NODE ENTITY_REFERENCE_NODE ENTITY_NODE ' +
    'PROCESSING_INSTRUCTION_NODE COMMENT_NODE DOCUMENT_NODE DOCUMENT_TYPE_NODE DOCUMENT_FRAGMENT_NODE ' +
    'NOTATION_NODE DOCUMENT_POSITION_DISCONNECTED DOCUMENT_POSITION_PRECEDING ' +
    'DOCUMENT_POSITION_FOLLOWING DOCUMENT_POSITION_CONTAINS DOCUMENT_POSITION_CONTAINED_BY ' +
    'DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC')
    .split(' ')
    .forEach(function (name, i) {
      var value = i < 12 ? i + 1 : 1 << (i - 12);
      [Node, Node.prototype].forEach(function (o) {
        Object.defineProperty(o, name, { value: value, enumerable: true });
      });
    });

  // CharacterData: `remove` (ChildNode, shared with Element and DocumentType in
  // elem2.js), then `data` and `length`.
  rw(g.CharacterData.prototype, 'data', function () {
    return data(this);
  }, setData);
  g.__frot_ifaceattrs(g.CharacterData.prototype, {
    length: function () {
      return data(this).length;
    },
  });
})(globalThis);
