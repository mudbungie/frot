// An element's attribute list as Firefox 153.0esr exposes it beyond
// get/set/remove (js.md §15: lit-html prepares every template through
// `hasAttributes`/`getAttributeNames`, binds boolean attributes with
// `toggleAttribute`, and component code walks `attributes`). The arena is the
// one home of the list (js.md §2): `attributes` is a live view over it, and an
// `Attr` is a handle naming one attribute of one element, made once per pair.
// Residual (js.md §15.3): an `Attr` answers its own members and `nodeType`,
// `nodeName`, `nodeValue` and `textContent`; the tree members of Node do not
// apply to it in frot.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;
  var proto = g.Element.prototype;

  function names(el) {
    return g.__frot_attrs(slots(el).id).map(function (a) {
      return a[0];
    });
  }

  // --- Attr: one per (element, name), remembered on the element --------------
  var Attr = g.__frot_extends(g.__frot_iface('Attr'), g.Node);
  function attrOf(el, name) {
    var st = slots(el);
    var made = st.attrNodes || (st.attrNodes = Object.create(null));
    if (!made[name]) {
      made[name] = Object.create(Attr.prototype);
      slots(made[name]).attr = { el: el, name: name, last: '' };
    }
    return made[name];
  }
  // The element still carrying this attribute, else null; reading it keeps
  // the value a removed attribute goes on answering (measured).
  function owner(a) {
    var v = a.el.getAttribute(a.name);
    if (v === null) return null;
    a.last = v;
    return a.el;
  }
  function state(attr) {
    return slots(attr).attr;
  }
  function value() {
    var a = state(this);
    owner(a);
    return a.last;
  }
  function setValue(v) {
    var a = state(this);
    if (owner(a)) a.el.setAttribute(a.name, v);
    else a.last = String(v);
  }
  g.__frot_ifaceattrs(Attr.prototype, {
    localName: function () {
      return state(this).name;
    },
  });
  Object.defineProperty(Attr.prototype, 'value', {
    get: brand(value, 'get value'),
    set: brand(setValue, 'set value'),
    enumerable: true,
    configurable: true,
  });
  g.__frot_ifaceattrs(Attr.prototype, {
    name: function () {
      return state(this).name;
    },
    namespaceURI: function () {
      return null;
    },
    prefix: function () {
      return null;
    },
    specified: function () {
      return true;
    },
    ownerElement: function () {
      return owner(state(this));
    },
  });

  // --- NamedNodeMap: a live view, the same object at every read --------------
  var NamedNodeMap = g.__frot_iface('NamedNodeMap');
  function el(map) {
    return slots(map).el;
  }
  g.__frot_ifaceops(NamedNodeMap.prototype, {
    getNamedItem: function getNamedItem(name) {
      var n = String(name).toLowerCase();
      return el(this).hasAttribute(n) ? attrOf(el(this), n) : null;
    },
    item: function item(i) {
      var n = names(el(this))[i >>> 0];
      return n === undefined ? null : attrOf(el(this), n);
    },
  });
  g.__frot_ifaceattrs(NamedNodeMap.prototype, {
    length: function () {
      return names(el(this)).length;
    },
  });
  Object.defineProperty(NamedNodeMap.prototype, Symbol.iterator, {
    value: Array.prototype.values,
    writable: true,
    configurable: true,
  });
  function index(key) {
    return typeof key === 'string' && /^(0|[1-9][0-9]*)$/.test(key) ? +key : -1;
  }
  function mapOf(element) {
    var st = slots(element);
    if (st.attrMap) return st.attrMap;
    var target = Object.create(NamedNodeMap.prototype);
    slots(target).el = element;
    // Indices are enumerable own properties, names non-enumerable ones, read
    // off the arena at each access (measured: `Object.keys` lists the indices,
    // `getOwnPropertyNames` both).
    st.attrMap = new Proxy(target, {
      get: function (t, key, recv) {
        var all = names(element);
        var i = index(key);
        if (i >= 0) return i < all.length ? attrOf(element, all[i]) : undefined;
        if (typeof key === 'string' && !(key in t) && all.indexOf(key) >= 0) return attrOf(element, key);
        return Reflect.get(t, key, recv);
      },
      has: function (t, key) {
        var all = names(element);
        var i = index(key);
        return i >= 0 ? i < all.length : all.indexOf(key) >= 0 || key in t;
      },
      ownKeys: function () {
        var all = names(element);
        return all.map(function (_n, i) {
          return String(i);
        }).concat(all);
      },
      getOwnPropertyDescriptor: function (t, key) {
        var all = names(element);
        var i = index(key);
        var n = i >= 0 ? all[i] : all.indexOf(key) >= 0 ? key : undefined;
        if (n === undefined) return Reflect.getOwnPropertyDescriptor(t, key);
        return { value: attrOf(element, n), writable: false, enumerable: i >= 0, configurable: true };
      },
    });
    slots(st.attrMap).el = element;
    return st.attrMap;
  }

  g.__frot_ifaceops(proto, {
    getAttributeNames: function getAttributeNames() {
      return names(this);
    },
    toggleAttribute: function toggleAttribute(name) {
      if (!arguments.length) {
        throw new TypeError('Element.toggleAttribute: At least 1 argument required, but only 0 passed');
      }
      var force = arguments[1];
      var n = String(name).toLowerCase();
      var has = this.hasAttribute(n);
      var want = force === undefined ? !has : !!force;
      if (want && !has) this.setAttribute(n, '');
      if (!want && has) this.removeAttribute(n);
      return want;
    },
    hasAttributes: function hasAttributes() {
      return names(this).length > 0;
    },
  });
  g.__frot_ifaceattrs(proto, {
    localName: function () {
      return g.__frot_tag(slots(this).id);
    },
    attributes: function () {
      return mapOf(this);
    },
  });
})(globalThis);
