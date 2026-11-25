// ElementInternals, ValidityState and CustomStateSet (js.md §15.6), shaped
// as Firefox 153.0esr answers them (js/tests/internals/). What the views read
// — form association, custom states, validity, ARIA defaults — is arena
// state (`dom::live::internals`), written here and read back, never a second
// copy; the validity flags and message are the internals' own. The
// form-associated reactions (form owner, disabled state) are facereact.js's.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var brand = g.__frot_brand;
  var err = g.__frot_domerror;
  var ce = g.__frot_ce;
  var EI = g.__frot_iface('ElementInternals');
  var VS = g.__frot_iface('ValidityState');
  var CSS = g.__frot_iface('CustomStateSet');
  var FLAGS = ['valueMissing', 'typeMismatch', 'patternMismatch', 'tooLong', 'tooShort', 'rangeUnderflow',
    'rangeOverflow', 'stepMismatch', 'badInput', 'customError'];

  // The internals (or state set) `o` is, after Gecko's brand check.
  function own(o, iface, member) {
    var st = slots(o);
    if (st.iface !== iface) {
      throw new TypeError("'" + member + "' called on an object that does not implement interface " + iface + '.');
    }
    return st;
  }
  // The internals of a form-associated element, or Firefox's refusal;
  // `member` is an operation's name or an attribute's `<name> getter`.
  function face(o, member) {
    var get = / getter$/.test(member);
    var st = own(o, 'ElementInternals', get ? 'get ' + member.slice(0, -7) : member);
    if (!st.face) {
      throw err('NotSupportedError', 'ElementInternals.' + member +
        ': Target element is not a form-associated custom element');
    }
    return st;
  }
  function id(st) {
    return slots(st.el).id;
  }
  function invalid(st) {
    return FLAGS.some(function (f) {
      return st.flags[f];
    });
  }
  // A candidate for constraint validation: not disabled, not readonly, not
  // in a datalist (HTML, barred from constraint validation).
  function candidate(st) {
    return !g.__frot_matches(id(st), ':disabled, [readonly], datalist *');
  }
  function check(st) {
    if (!candidate(st) || !invalid(st)) return true;
    g.EventTarget.prototype.dispatchEvent.call(st.el, new g.Event('invalid', { cancelable: true }));
    return false;
  }
  function getter(name, f) {
    return brand(f, 'get ' + name);
  }

  g.__frot_ifaceops(EI.prototype, {
    setFormValue: function setFormValue() {
      face(this, 'setFormValue');
      if (!arguments.length) {
        throw new TypeError('ElementInternals.setFormValue: At least 1 argument required, but only 0 passed');
      }
      // frot submits no form, so the value has no reader (js.md §15.6).
    },
    setValidity: function setValidity(flags, message, anchor) {
      var st = face(this, 'setValidity');
      var o = flags == null ? {} : Object(flags);
      var next = {};
      FLAGS.forEach(function (f) {
        next[f] = !!o[f];
      });
      var any = FLAGS.some(function (f) {
        return next[f];
      });
      if (any && (message === undefined || String(message) === '')) {
        throw new TypeError('ElementInternals.setValidity: Need to provide validation message');
      }
      if (anchor != null && !inside(slots(anchor).id, id(st))) {
        throw err('NotFoundError', 'ElementInternals.setValidity: Validation anchor is not a shadow-including ' +
          'inclusive descendant of target element');
      }
      st.flags = next;
      st.message = any ? String(message) : '';
      g.__frot_set_invalid(id(st), any);
    },
    checkValidity: function checkValidity() {
      return check(face(this, 'checkValidity'));
    },
    reportValidity: function reportValidity() {
      return check(face(this, 'reportValidity'));
    },
  });
  // Whether `n` is `host` or inside it, through shadow roots.
  function inside(n, host) {
    while (n !== null && n !== undefined && n !== host) {
      var info = g.__frot_shadow_info(n);
      n = info ? info[0] : g.__frot_parent(n);
    }
    return n === host;
  }
  g.__frot_ifaceattrs(EI.prototype, {
    shadowRoot: function () {
      var r = g.__frot_shadow(id(own(this, 'ElementInternals', 'get shadowRoot')));
      return r === null || r === undefined ? null : wrap(r);
    },
    form: function () {
      var o = face(this, 'form getter').el;
      return slots(o).owner == null ? null : wrap(slots(o).owner);
    },
    willValidate: function () {
      return candidate(face(this, 'willValidate getter'));
    },
    validity: function () {
      var st = face(this, 'validity getter');
      if (!st.validity) {
        st.validity = Object.create(VS.prototype);
        slots(st.validity).iface = 'ValidityState';
        slots(st.validity).of = st;
      }
      return st.validity;
    },
    validationMessage: function () {
      var st = face(this, 'validationMessage getter');
      return candidate(st) && invalid(st) ? st.message : '';
    },
    labels: function () {
      var st = face(this, 'labels getter');
      var ids = g.__frot_labels(id(st));
      // The same NodeList while the labels are the same (Firefox's is live).
      if (!st.labels || st.labelIds !== ids.join()) {
        st.labels = g.__frot_nodelist(ids);
        st.labelIds = ids.join();
      }
      return st.labels;
    },
    states: function () {
      var st = own(this, 'ElementInternals', 'get states');
      if (!st.states) {
        st.states = Object.create(CSS.prototype);
        slots(st.states).iface = 'CustomStateSet';
        slots(st.states).el = st.el;
      }
      return st.states;
    },
  });

  // ARIAMixin on the internals: the default each attribute takes where the
  // author set none, which `ax` reads (Chrome 158, fidelity `internals`).
  // The element-reference members are held here and reach no view.
  var ARIA = ('ActiveDescendantElement role Atomic AutoComplete BrailleLabel BrailleRoleDescription Busy ' +
    'Checked ColCount ColIndex ColIndexText ColSpan ControlsElements Current DescribedByElements ' +
    'Description DetailsElements Disabled ErrorMessageElements Expanded FlowToElements HasPopup Hidden ' +
    'Invalid KeyShortcuts Label LabelledByElements Level Live Modal MultiLine MultiSelectable Orientation ' +
    'OwnsElements Placeholder PosInSet Pressed ReadOnly Relevant Required RoleDescription RowCount ' +
    'RowIndex RowIndexText RowSpan Selected SetSize Sort ValueMax ValueMin ValueNow ValueText').split(' ');
  ARIA.forEach(function (k) {
    var prop = k === 'role' ? k : 'aria' + k;
    var attr = k === 'role' ? k : 'aria-' + k.toLowerCase();
    var refs = /Elements?$/.test(k);
    Object.defineProperty(EI.prototype, prop, {
      get: getter(prop, function () {
        var st = own(this, 'ElementInternals', 'get ' + prop);
        if (refs) return st.refs && st.refs[prop] !== undefined ? st.refs[prop] : null;
        var v = g.__frot_aria_default(id(st), attr);
        return v === undefined ? null : v;
      }),
      set: brand(function (v) {
        var st = own(this, 'ElementInternals', 'set ' + prop);
        if (refs) {
          (st.refs || (st.refs = {}))[prop] = v == null ? null : v;
          return;
        }
        g.__frot_set_aria_default(id(st), attr, v == null ? null : String(v));
      }, 'set ' + prop),
      enumerable: true,
      configurable: true,
    });
  });

  g.__frot_ifaceattrs(VS.prototype, FLAGS.concat('valid').reduce(function (o, f) {
    o[f] = function () {
      var st = own(this, 'ValidityState', 'get ' + f).of;
      return f === 'valid' ? !invalid(st) : !!st.flags[f];
    };
    return o;
  }, {}));

  // --- CustomStateSet: a setlike over the arena's states ----------------------
  function states(o, member) {
    return slots(own(o, 'CustomStateSet', member).el).id;
  }
  function list(o, member) {
    return g.__frot_states(states(o, member));
  }
  // Change the states of `o` by `f(list)`; answers what `f` did.
  function change(o, member, f) {
    var n = states(o, member);
    var s = g.__frot_states(n);
    var did = f(s);
    g.__frot_set_states(n, s);
    return did;
  }
  var A = Array.prototype;
  g.__frot_ifaceops(CSS.prototype, {
    add: function add(v) {
      v = String(v);
      change(this, 'add', function (s) {
        if (s.indexOf(v) < 0) s.push(v);
      });
    },
    delete: function delete_(v) {
      v = String(v);
      return change(this, 'delete', function (s) {
        var i = s.indexOf(v);
        if (i >= 0) s.splice(i, 1);
        return i >= 0;
      });
    },
    clear: function clear() {
      change(this, 'clear', function (s) {
        s.length = 0;
      });
    },
    entries: function entries() {
      return A.map.call(list(this, 'entries'), function (v) {
        return [v, v];
      }).values();
    },
    keys: function keys() {
      return list(this, 'keys').values();
    },
    values: function values() {
      return list(this, 'values').values();
    },
    forEach: function forEach(cb, self) {
      var set = this;
      list(this, 'forEach').forEach(function (v) {
        cb.call(self, v, v, set);
      });
    },
    has: function has(v) {
      return list(this, 'has').indexOf(String(v)) >= 0;
    },
  });
  g.__frot_ifaceattrs(CSS.prototype, {
    size: function () {
      return list(this, 'size').length;
    },
  });
  Object.defineProperty(CSS.prototype, Symbol.iterator, {
    value: CSS.prototype.values,
    writable: true,
    configurable: true,
  });

  // --- HTMLElement.attachInternals ---------------------------------------------
  var at = 'HTMLElement.attachInternals: ';
  g.__frot_ifaceops(g.HTMLElement.prototype, {
    attachInternals: function attachInternals() {
      var st = slots(this);
      var local = g.__frot_tag(st.id);
      if (!ce.valid(local) || ce.isValues.has(st.id)) {
        throw err('NotSupportedError', at + "Cannot attach ElementInternals to a customized built-in or " +
          "non-custom element '" + local + "'");
      }
      var def = ce.defs[local];
      if (!def || st.failed) {
        throw err('NotSupportedError', at + "Cannot attach ElementInternals to a non-custom element '" + local + "'");
      }
      if (def.noInternals) {
        throw err('NotSupportedError', at + "AttachInternal() to '" + local + "' is disabled by disabledFeatures");
      }
      if (st.internals) {
        throw err('NotSupportedError', at + "AttachInternals() has already been called from '" + local + "'");
      }
      var i = Object.create(EI.prototype);
      var is = slots(i);
      is.iface = 'ElementInternals';
      is.el = this;
      is.face = def.face;
      is.flags = {};
      is.message = '';
      st.internals = i;
      return i;
    },
  });
  // Gecko lists it second, after `click`: move every later member after it.
  var H = g.HTMLElement.prototype;
  Object.getOwnPropertyNames(H).forEach(function (k) {
    if (k === 'click' || k === 'attachInternals') return;
    var d = Object.getOwnPropertyDescriptor(H, k);
    delete H[k];
    Object.defineProperty(H, k, d);
  });
})(globalThis);
