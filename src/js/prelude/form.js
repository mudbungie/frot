// Forms and the document's address (bl-09c8), each value as Firefox 153.0esr
// answers it (src/js/tests/form.rs): `document.URL`/`documentURI`,
// `HTMLElement.title`, `document.forms`, a form's `elements`/`length`/`action`,
// and every form-associated element's `form`. Nothing is cached: the form
// owner is recomputed from the arena on every read, so a moved control or an
// edited `form` attribute is seen at once.
(function (g) {
  'use strict';
  var tagdef = g.__frot_tagdef;
  var D = g.Document.prototype;
  var hasOwn = Object.prototype.hasOwnProperty;

  // The document's URL is the window's address; frot never moves it apart
  // from `location` (a pushState leaves both where they were, envobj.js).
  g.__frot_ifaceattrs(D, {
    URL: function () {
      return g.location.href;
    },
    documentURI: function () {
      return g.location.href;
    },
    forms: function () {
      return named(g.__frot_elems(ids(this.querySelectorAll('form'))), first);
    },
  });
  Object.defineProperty(g.HTMLElement.prototype, 'title', g.__frot_reflect('title'));

  function ids(list) {
    return Array.prototype.map.call(list, function (n) {
      return g.__frot_slots(n).id;
    });
  }

  // The form owner: the arena's one answer (src/dom/form.rs — a connected
  // element's `form` attribute names it by id, anything but a form there is
  // no owner, else the nearest form ancestor). The parser's own association
  // for a control foster-parented out of its form is not kept: a declared
  // residual.
  function owner(el) {
    var f = g.__frot_form_owner(g.__frot_slots(el).id);
    return f === null || f === undefined ? null : g.__frot_wrap(f);
  }
  var formOwner = { get: function () { return owner(this); } };
  tagdef('Button FieldSet Form Input Object Output Select TextArea', 'name', g.__frot_reflect('name'));
  tagdef('Button FieldSet Input Object Output Select TextArea', 'form', formOwner);
  // A legend reports its fieldset's form, an option its select's, a label its
  // control's — each null when there is none.
  tagdef('Legend', 'form', { get: function () {
    var p = this.parentElement;
    return p && p.localName === 'fieldset' ? p.form : null;
  } });
  tagdef('Option', 'form', { get: function () {
    var s = this.closest('select');
    return s ? s.form : null;
  } });
  tagdef('Label', 'form', { get: function () {
    var id = this.getAttribute('for');
    var c = id !== null ? this.ownerDocument.getElementById(id)
      : this.querySelector('button,input,meter,output,progress,select,textarea');
    return c && c.form !== undefined ? c.form : null;
  } });

  // The listed elements a form owns, in tree order; an image button is not one.
  var LISTED = 'button,fieldset,input,object,output,select,textarea';
  function controls(form) {
    return Array.prototype.filter.call(form.getRootNode().querySelectorAll(LISTED), function (el) {
      return owner(el) === form && !(el.localName === 'input' && /^image$/i.test(el.getAttribute('type')));
    });
  }

  // HTMLFormControlsCollection and RadioNodeList: a name several controls
  // share reads as the list of them, one as the control itself.
  var HFCC = g.__frot_iface('HTMLFormControlsCollection');
  g.__frot_extends(HFCC, g.HTMLCollection);
  var RNL = g.__frot_iface('RadioNodeList');
  g.__frot_extends(RNL, g.NodeList);
  function group(list) {
    return list.length > 1 ? Object.setPrototypeOf(g.__frot_nodelist(ids(list)), RNL.prototype)
      : list[0];
  }
  function first(list) {
    return list[0];
  }
  // A snapshot's named properties: every id and name of its elements, first
  // seen first, unenumerable as WebIDL's [LegacyUnenumerableNamedProperties]
  // makes them, never over an index or a prototype member.
  function named(c, pick) {
    var seen = Object.create(null), order = [];
    for (var i = 0; hasOwn.call(c, i); i++) {
      [c[i].getAttribute('id'), c[i].getAttribute('name')].forEach(function (k) {
        if (!k || (seen[k] && seen[k].indexOf(c[i]) >= 0)) return;
        if (!seen[k]) order.push(k);
        (seen[k] = seen[k] || []).push(c[i]);
      });
    }
    order.forEach(function (k) {
      if (!(k in c)) Object.defineProperty(c, k, { value: pick(seen[k]), configurable: true });
    });
    return c;
  }
  g.__frot_ifaceops(HFCC.prototype, {
    namedItem: function namedItem(name) {
      var s = String(name), hits = [];
      for (var i = 0; hasOwn.call(this, i); i++) {
        if (this[i].getAttribute('id') === s || this[i].getAttribute('name') === s) hits.push(this[i]);
      }
      return hits.length ? group(hits) : null;
    },
  });
  // `value` is the checked radio's value; setting it checks the radio that has it.
  function radios(list) {
    return Array.prototype.filter.call(list, function (r) {
      return r.localName === 'input' && /^radio$/i.test(r.getAttribute('type'));
    });
  }
  function radioValue(r) {
    return r.hasAttribute('value') ? r.getAttribute('value') : 'on';
  }
  Object.defineProperty(RNL.prototype, 'value', {
    get: g.__frot_brand(function () {
      var r = radios(this).filter(function (x) { return x.checked; })[0];
      return r ? radioValue(r) : '';
    }, 'get value'),
    set: g.__frot_brand(function (v) {
      var r = radios(this).filter(function (x) { return radioValue(x) === String(v); })[0];
      if (r) r.checked = true;
    }, 'set value'),
    enumerable: true,
    configurable: true,
  });

  function elements(form) {
    var c = Object.setPrototypeOf(g.__frot_elems(ids(controls(form))), HFCC.prototype);
    return named(c, group);
  }
  tagdef('Form', 'elements', { get: function () { return elements(this); } });
  tagdef('Form', 'length', { get: function () { return controls(this).length; } });
  // `action` resolves against the document; absent or empty, it is the
  // document's own URL.
  var url = g.__frot_reflect_url('action');
  tagdef('Form', 'action', { get: function () {
    return this.getAttribute('action') ? url.get.call(this) : this.ownerDocument.URL;
  }, set: url.set });
  // A fieldset's are every listed descendant, image buttons included.
  tagdef('FieldSet', 'elements', { get: function () {
    return g.__frot_elems(ids(this.querySelectorAll(LISTED)));
  } });
})(globalThis);
