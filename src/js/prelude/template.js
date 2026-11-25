// HTMLTemplateElement (js.md §15): `content` is the fragment the arena gives
// every template — filled by the parser, empty for a created one — and the
// five `shadowRoot*` attributes reflect the declarative-shadow-root content
// attributes, as Firefox 153.0esr shapes them. The contents are not children:
// nothing that walks the tree enters them, and markup.js serializes them in a
// template's place.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;
  var P = g.HTMLTemplateElement.prototype;

  // An enumerated attribute: the keyword its value names (ASCII
  // case-insensitively), else `missing` — which also covers an invalid value.
  function keyword(attr, keys, missing) {
    return {
      get: function () {
        var v = (this.getAttribute(attr) || '').toLowerCase();
        return keys.indexOf(v) >= 0 ? v : missing;
      },
      set: function (v) {
        this.setAttribute(attr, String(v));
      },
    };
  }
  // A boolean attribute: present or not; set writes it empty, as Firefox does.
  function flag(attr) {
    return {
      get: function () {
        return this.hasAttribute(attr);
      },
      set: function (v) {
        if (v) this.setAttribute(attr, '');
        else this.removeAttribute(attr);
      },
    };
  }

  g.__frot_ifaceattrs(P, {
    content: function () {
      return g.__frot_wrap(g.__frot_content(slots(this).id));
    },
  });
  var members = {
    shadowRootMode: keyword('shadowrootmode', ['open', 'closed'], ''),
    shadowRootDelegatesFocus: flag('shadowrootdelegatesfocus'),
    shadowRootClonable: flag('shadowrootclonable'),
    shadowRootSerializable: flag('shadowrootserializable'),
    shadowRootSlotAssignment: keyword('shadowrootslotassignment', ['named', 'manual'], 'named'),
  };
  Object.keys(members).forEach(function (key) {
    var d = members[key];
    Object.defineProperty(P, key, {
      get: brand(d.get, 'get ' + key),
      set: brand(d.set, 'set ' + key),
      enumerable: true,
      configurable: true,
    });
  });
})(globalThis);
