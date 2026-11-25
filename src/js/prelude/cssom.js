// Constructable stylesheets and `adoptedStyleSheets` (js.md §15) — what Lit
// and its kind use to style a shadow tree: `new CSSStyleSheet()`,
// `replaceSync(text)`, and `root.adoptedStyleSheets = [sheet]`. A sheet is
// its text; what a root has adopted is written to the arena
// (`__frot_adopt`, `dom::live`), where the cascade reads it, every time
// either changes. Residual (js.md §15.3): `cssRules` lists the sheet's
// top-level rules as written (Firefox re-serializes them), each a plain
// `CSSRule` with `cssText`; there is no `document.styleSheets`.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;
  var err = g.__frot_domerror;

  // The top-level rules of `text`, by brace depth; `@import` is dropped, as
  // a constructed sheet drops it (measured).
  function split(text) {
    var out = [];
    var depth = 0;
    var start = 0;
    for (var i = 0; i < text.length; i++) {
      var c = text[i];
      if (c === '{') depth++;
      else if (c === '}' && depth > 0 && --depth === 0) {
        out.push(text.slice(start, i + 1).trim());
        start = i + 1;
      } else if (c === ';' && depth === 0) start = i + 1;
    }
    return out.filter(function (r) {
      return !/^@import/i.test(r);
    });
  }

  var StyleSheet = g.__frot_iface('StyleSheet');
  g.__frot_ifaceattrs(StyleSheet.prototype, {
    type: function () {
      return 'text/css';
    },
    href: function () {
      return null;
    },
    ownerNode: function () {
      return null;
    },
    parentStyleSheet: function () {
      return null;
    },
    title: function () {
      return null;
    },
    media: function () {
      return { length: 0, mediaText: '' };
    },
  });
  g.__frot_rwattrs(StyleSheet.prototype, ['disabled']);
  var CSSRule = g.__frot_iface('CSSRule');
  g.__frot_ifaceattrs(CSSRule.prototype, ['cssText']);
  var CSSRuleList = g.__frot_iface('CSSRuleList');
  g.__frot_ifaceops(CSSRuleList.prototype, {
    item: function item(i) {
      var r = this[i >>> 0];
      return r === undefined ? null : r;
    },
  });
  g.__frot_ifaceattrs(CSSRuleList.prototype, ['length']);

  var CSSStyleSheet = g.__frot_extends(g.__frot_iface('CSSStyleSheet', null, function (inst) {
    var st = slots(inst);
    st.rules = [];
    st.roots = new Set();
    st.disabled = false;
  }), StyleSheet);
  function changed(sheet) {
    slots(sheet).roots.forEach(sync);
  }
  g.__frot_ifaceops(CSSStyleSheet.prototype, {
    insertRule: function insertRule(rule) {
      var rules = slots(this).rules;
      var at = arguments.length > 1 ? arguments[1] >>> 0 : 0;
      if (at > rules.length) {
        throw err('IndexSizeError', 'Index or size is negative or greater than the allowed amount');
      }
      rules.splice(at, 0, String(rule).trim());
      changed(this);
      return at;
    },
    deleteRule: function deleteRule(at) {
      var rules = slots(this).rules;
      if (at >>> 0 >= rules.length) {
        throw err('IndexSizeError', 'Index or size is negative or greater than the allowed amount');
      }
      rules.splice(at >>> 0, 1);
      changed(this);
    },
    replace: function replace(text) {
      this.replaceSync(text);
      return Promise.resolve(this);
    },
    replaceSync: function replaceSync(text) {
      slots(this).rules = split(String(text));
      changed(this);
    },
  });
  function cssRules() {
    var list = Object.create(CSSRuleList.prototype);
    var rules = slots(this).rules;
    rules.forEach(function (text, i) {
      var rule = Object.create(CSSRule.prototype);
      slots(rule).cssText = text;
      Object.defineProperty(list, i, { value: rule, enumerable: true });
    });
    slots(list).length = rules.length;
    return list;
  }
  g.__frot_ifaceattrs(CSSStyleSheet.prototype, {
    ownerRule: function () {
      return null;
    },
    cssRules: cssRules,
    rules: cssRules,
  });

  // --- adoptedStyleSheets on the document and every shadow root --------------
  var at = { document: 'Document', shadow: 'ShadowRoot' };
  function sync(root) {
    var list = slots(g.__frot_wrap(root)).adopted || [];
    g.__frot_adopt(root, list.map(function (s) {
      return slots(s).rules.join('\n');
    }));
  }
  function check(where, sheet) {
    if (Object(sheet) !== sheet) {
      throw new TypeError(where + '.adoptedStyleSheets setter: Element of value being assigned is not an object.');
    }
    if (!(sheet instanceof CSSStyleSheet)) {
      throw new TypeError(where + '.adoptedStyleSheets setter: Element of value being assigned does not ' +
        'implement interface CSSStyleSheet.');
    }
  }
  // The array a root answers: a real one, whose writes are seen.
  function observed(root) {
    var st = slots(root);
    var id = st.id;
    var where = root === g.document ? at.document : at.shadow;
    var target = [];
    return new Proxy(target, {
      set: function (t, key, value) {
        if (key !== 'length') check(where, value);
        t[key] = value;
        if (key !== 'length') slots(value).roots.add(id);
        st.adopted = t.slice();
        sync(id);
        return true;
      },
      deleteProperty: function (t, key) {
        delete t[key];
        st.adopted = t.filter(Boolean);
        sync(id);
        return true;
      },
    });
  }
  function adopted() {
    var st = slots(this);
    return st.view || (st.view = observed(this));
  }
  function adopt(value) {
    var where = this === g.document ? at.document : at.shadow;
    if (Object(value) !== value || typeof value[Symbol.iterator] !== 'function') {
      throw new TypeError(where + ".adoptedStyleSheets setter: Value being assigned can't be converted to a " +
        'observable array.');
    }
    var list = Array.from(value);
    list.forEach(function (s) {
      check(where, s);
    });
    var view = adopted.call(this);
    view.length = 0;
    list.forEach(function (s) {
      view.push(s);
    });
  }
  [g.Document.prototype, g.ShadowRoot.prototype].forEach(function (P) {
    Object.defineProperty(P, 'adoptedStyleSheets', {
      get: brand(adopted, 'get adoptedStyleSheets'),
      set: brand(adopt, 'set adoptedStyleSheets'),
      enumerable: true,
      configurable: true,
    });
  });
})(globalThis);
