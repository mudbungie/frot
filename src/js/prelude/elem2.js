// The members of Element the frameworks lean on (js.md §3), split from elem.js
// to stay under the 300-line source cap: attributes, selectors and collections
// (querySelector/getElementsBy*/matches/closest), the ParentNode/ChildNode
// mixins (ParentNode on DocumentFragment too), namespaced attributes, and the
// form-control reflections on the
// HTML<Tag>Element interfaces that own them (bl-78cb). Loads after elem.js
// (needs its one reflection maker).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var proto = g.Element.prototype;
  var tagdef = g.__frot_tagdef;

  function query(el, sel) {
    return g.__frot_query(slots(el).id, sel);
  }
  // The ParentNode/ChildNode convenience mixin (bl-3a36): append/prepend take
  // nodes or strings (a string becomes a text node, per spec); remove detaches
  // from the parent. All three ride the existing insertion/removal paths.
  function toNode(v) {
    return v && v.nodeType ? v : g.document.createTextNode(String(v));
  }
  function remove() {
    g.__frot_detach(slots(this).id);
  }

  g.__frot_ifaceops(proto, {
    getAttribute: function getAttribute(name) {
      // The arena stores attribute names ASCII-lowercased (the parser and
      // set_attr both normalize), so reads normalize too — SVG's camelCase
      // (viewBox) round-trips instead of silently missing (bl-3a36).
      var v = g.__frot_attr(slots(this).id, String(name).toLowerCase());
      return v === null || v === undefined ? null : v;
    },
    // Namespaced attributes (bl-3a36): the arena stores plain attribute names
    // (the parser flattens foreign-content attrs the same way), so the NS
    // variants delegate — React sets xlink:href/xml:lang through setAttributeNS.
    getAttributeNS: function getAttributeNS(_ns, name) {
      return this.getAttribute(name);
    },
    setAttribute: function setAttribute(name, value) {
      g.__frot_set_attr(slots(this).id, name, String(value));
    },
    setAttributeNS: function setAttributeNS(_ns, name, value) {
      this.setAttribute(name, value);
    },
    removeAttribute: function removeAttribute(name) {
      g.__frot_remove_attr(slots(this).id, name);
    },
    removeAttributeNS: function removeAttributeNS(_ns, name) {
      this.removeAttribute(name);
    },
    hasAttribute: function hasAttribute(name) {
      return this.getAttribute(name) !== null;
    },
    closest: function closest(sel) {
      for (var n = this; n && n.nodeType === 1; n = n.parentNode) if (n.matches(sel)) return n;
      return null;
    },
    matches: function matches(sel) {
      return g.__frot_matches(slots(this).id, sel);
    },
    remove: remove,
  });
  // ChildNode is mixed into CharacterData and DocumentType too (measured).
  g.__frot_ifaceops(g.CharacterData.prototype, { remove: remove });
  g.__frot_ifaceops(g.DocumentType.prototype, { remove: remove });
  g.__frot_ifaceattrs(
    g.DocumentType.prototype,
    ['name', 'publicId', 'systemId'].reduce(function (o, k, i) {
      o[k] = function () {
        return g.__frot_doctype(slots(this).id)[i];
      };
      return o;
    }, {})
  );

  g.__frot_ifaceattrs(proto, {
    tagName: function () {
      return g.__frot_tag(slots(this).id).toUpperCase();
    },
  });

  // getElementsBy* return an HTMLCollection (the spec-named interface, built
  // by dom.js's one collection maker over the same selector hits qSA finds).
  // On Element and on Document, each with its own copies.
  [proto, g.Document.prototype].forEach(function (P) {
    g.__frot_ifaceops(P, {
      getElementsByTagName: function getElementsByTagName(tag) {
        return g.__frot_elems(query(this, String(tag)));
      },
      getElementsByClassName: function getElementsByClassName(cls) {
        return g.__frot_elems(query(this, '.' + String(cls).trim().split(/\s+/).join('.')));
      },
    });
  });

  // ParentNode: each interface that mixes it in owns its own copies (measured:
  // `Element.prototype.append !== DocumentFragment.prototype.append`); a
  // fragment and a document have `getElementById` where an element has none.
  function parentNode(P, byId) {
    g.__frot_ifaceops(P, byId ? {
      getElementById: function getElementById(id) {
        return this.querySelector('#' + id);
      },
    } : {});
    g.__frot_ifaceops(P, {
      prepend: function prepend() {
        var ref = this.firstChild;
        for (var i = 0; i < arguments.length; i++) this.insertBefore(toNode(arguments[i]), ref);
      },
      append: function append() {
        for (var i = 0; i < arguments.length; i++) this.appendChild(toNode(arguments[i]));
      },
      querySelector: function querySelector(sel) {
        var hits = query(this, sel);
        return hits.length ? wrap(hits[0]) : null;
      },
      querySelectorAll: function querySelectorAll(sel) {
        return g.__frot_nodelist(query(this, sel));
      },
    });
    function elements(n) {
      return g.__frot_children(slots(n).id).filter(function (id) {
        return g.__frot_kind(id) === 'element';
      });
    }
    g.__frot_ifaceattrs(P, {
      children: function () {
        return g.__frot_elems(elements(this));
      },
      firstElementChild: function () {
        return wrap(elements(this)[0]);
      },
      lastElementChild: function () {
        var ids = elements(this);
        return wrap(ids[ids.length - 1]);
      },
      childElementCount: function () {
        return elements(this).length;
      },
    });
  }
  parentNode(proto, false);
  parentNode(g.DocumentFragment.prototype, true);
  parentNode(g.Document.prototype, true);
  // NonDocumentTypeChildNode (bl-59f7): the nearest element sibling either
  // way, on Element and CharacterData — a doctype has neither (Firefox 153.0esr).
  function elementSibling(step) {
    return function () {
      var id = slots(this).id, p = g.__frot_parent(id);
      var ids = p === null || p === undefined ? [] : g.__frot_children(p);
      for (var j = ids.indexOf(id) + step; j >= 0 && j < ids.length; j += step) {
        if (g.__frot_kind(ids[j]) === 'element') return wrap(ids[j]);
      }
      return null;
    };
  }
  [proto, g.CharacterData.prototype].forEach(function (P) {
    g.__frot_ifaceattrs(P, {
      previousElementSibling: elementSibling(-1),
      nextElementSibling: elementSibling(1),
    });
  });

  // The form-control reflections frameworks/jQuery read off cloned nodes, each
  // on the interfaces Firefox 153.0esr defines that name on (bl-78cb; a `<div>`
  // has no `value`). Each mirrors an attribute, so the arena stays the one
  // source (no cached state). Radix-style libraries read
  // `Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set`
  // and call it (bl-3a36), which is why these are real own accessors there.
  function reflectBool(name) {
    return {
      get: function () {
        return this.hasAttribute(name);
      },
      set: function (v) {
        if (v) this.setAttribute(name, name);
        else this.removeAttribute(name);
      },
    };
  }
  // `value` reads a <textarea>'s text and any other control's attribute.
  // React sets node.defaultValue when it mounts a controlled <input>; frot
  // caches no form state, so `defaultValue` reflects the same `value` attribute.
  function value() {
    return {
      get: function () {
        return this.tagName === 'TEXTAREA' ? this.textContent : this.getAttribute('value') || '';
      },
      set: function (v) {
        this.setAttribute('value', String(v));
      },
    };
  }
  // Checkedness, selectedness and `indeterminate` are state, not attributes:
  // the arena keeps what a script sets (dom/live.rs, bl-ff74), and the
  // attribute is the default — `defaultChecked`, `defaultSelected`.
  function live(native) {
    return {
      get: function () {
        return native(slots(this).id);
      },
      set: function (v) {
        native(slots(this).id, !!v);
      },
    };
  }
  tagdef('Input', 'defaultChecked', reflectBool('checked'));
  tagdef('Input', 'checked', live(g.__frot_picked));
  tagdef('Details', 'open', reflectBool('open'));
  tagdef('Button Select OptGroup Option Link Input TextArea FieldSet Style', 'disabled', reflectBool('disabled'));
  tagdef('Input', 'indeterminate', live(g.__frot_indeterminate));
  tagdef('Option', 'defaultSelected', reflectBool('selected'));
  tagdef('Option', 'selected', live(g.__frot_picked));
  tagdef('Button Select Output LI Progress Data Option Input TextArea Meter Param', 'value', value());
  tagdef('Output Input TextArea', 'defaultValue', value());
  // `type` is a plain reflected string, from elem.js's one maker (bl-d313).
  tagdef(
    'Area Button Object LI Script UList OList Anchor Embed Link Input Style Source Param',
    'type',
    g.__frot_reflect('type')
  );
  // On these four `type` is read-only and names the control (measured:
  // 'fieldset', 'output', 'textarea', 'select-one' | 'select-multiple').
  tagdef('FieldSet Output TextArea Select', 'type', {
    get: function () {
      var tag = g.__frot_tag(slots(this).id);
      if (tag !== 'select') return tag;
      return this.hasAttribute('multiple') ? 'select-multiple' : 'select-one';
    },
  });
})(globalThis);
