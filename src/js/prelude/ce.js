// The custom element registry (js.md §15): `customElements`, the
// [HTMLConstructor] behaviour of the HTML element interfaces dom.js builds,
// and `createElement` of a defined name. The reactions — queues, upgrade, the
// mutation seam — are cereact.js's, whose state this file shares.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var brand = g.__frot_brand;
  var err = g.__frot_domerror;
  var ce = g.__frot_ce;
  var defs = ce.defs;
  var ctors = ce.ctors;
  var valid = ce.valid;
  var waiting = Object.create(null); // name -> [promise, resolve]
  var defining = false;

  // [HTMLConstructor]: `super()` from a defined class. During an upgrade it
  // answers the element being upgraded, re-prototyped, so a page's reference
  // to it stays the same object; otherwise it makes a new one.
  function construct(target, iface) {
    // Measured on Firefox 153.0esr: an interface no definition names throws
    // "Illegal constructor." — except HTMLElement once anything is defined,
    // which throws Gecko's bare NS_ERROR_UNEXPECTED, as a definition reached
    // through the wrong interface does.
    var def = ctors.get(target);
    if (!def && !(ce.count && iface === 'HTMLElement')) throw new TypeError('Illegal constructor.');
    if (!def || iface !== def.iface) throw err('NS_ERROR_UNEXPECTED', '');
    var top = ce.building[ce.building.length - 1];
    if (top && top[0] === def) {
      Object.setPrototypeOf(top[1], target.prototype);
      return top[1];
    }
    var w = wrap(ce.raw.create_element(def.local));
    Object.setPrototypeOf(w, target.prototype);
    if (def.local !== def.name) ce.isValues.set(slots(w).id, def.name);
    ce.made(w, def);
    return w;
  }

  // --- The registry ----------------------------------------------------------
  var CER = g.__frot_iface('CustomElementRegistry');
  var at = 'CustomElementRegistry.';
  function need(op, n, got) {
    if (got < n) throw new TypeError(at + op + ': At least ' + n + ' argument' + (n > 1 ? 's' : '') +
      ' required, but only ' + got + ' passed');
  }
  function constructor_(f) {
    try {
      Reflect.construct(String, [], f);
      return true;
    } catch (e) {
      return false;
    }
  }
  var CALLBACKS = ['connectedCallback', 'disconnectedCallback', 'adoptedCallback', 'attributeChangedCallback'];
  var FORM_CALLBACKS = ['formAssociatedCallback', 'formResetCallback', 'formDisabledCallback',
    'formStateRestoreCallback'];
  g.__frot_ifaceops(CER.prototype, {
    define: function define(name, ctor) {
      var options = arguments[2];
      need('define', 2, arguments.length);
      if (typeof ctor !== 'function' || !constructor_(ctor)) {
        throw new TypeError(at + 'define: Argument 2 is not a constructor.');
      }
      name = String(name);
      if (!valid(name)) throw err('SyntaxError', at + "define: '" + name + "' is not a valid custom element name");
      if (defs[name]) {
        throw err('NotSupportedError', at + "define: '" + name + "' has already been defined as a custom element");
      }
      if (ctors.has(ctor)) {
        throw err('NotSupportedError', at + "define: '" + name + "' and '" + ctors.get(ctor).name +
          "' have the same constructor");
      }
      var local = name;
      var face = 'HTMLElement';
      var ext = options && options.extends;
      if (ext !== undefined && ext !== null) {
        ext = String(ext);
        if (valid(ext)) throw err('NotSupportedError', at + "define: '" + name + "' cannot extend a custom element");
        var proto = Object.getPrototypeOf(g.document.createElement(ext));
        if (proto === g.HTMLUnknownElement.prototype) throw err('NotSupportedError', 'Operation is not supported');
        local = ext;
        face = proto[Symbol.toStringTag];
      }
      if (defining) {
        throw err('NotSupportedError', at + 'define: Cannot define a custom element while defining another custom element');
      }
      defining = true;
      var def = { name: name, local: local, iface: face, ctor: ctor, callbacks: {}, observed: [] };
      try {
        var prototype = ctor.prototype;
        if (Object(prototype) !== prototype) throw new TypeError(at + 'define: constructor.prototype is not an object.');
        var take = function (k) {
          var f = prototype[k];
          if (f === undefined) return;
          if (typeof f !== 'function') throw new TypeError("'" + k + "' member of LifecycleCallbacks is not an object.");
          def.callbacks[k] = f;
        };
        CALLBACKS.forEach(take);
        if (def.callbacks.attributeChangedCallback) {
          var obs = ctor.observedAttributes;
          if (obs !== undefined) def.observed = Array.from(obs, String);
        }
        // HTML's define, in its order: the disabled features, then whether
        // the element is form-associated and, if so, its form callbacks.
        var off = ctor.disabledFeatures;
        def.noInternals = off !== undefined && Array.from(off, String).indexOf('internals') >= 0;
        def.face = !!ctor.formAssociated;
        if (def.face) FORM_CALLBACKS.forEach(take);
      } finally {
        defining = false;
      }
      defs[name] = def;
      ctors.set(ctor, def);
      ce.count++;
      if (def.face) ce.faces++;
      ce.scope(function () {
        ce.each(g.__frot_document, function (w) {
          if (!slots(w).custom && ce.lookup(w) === def && g.__frot_connected(slots(w).id)) ce.enqueue(w, ce.UPGRADE);
        });
      });
      if (waiting[name]) {
        waiting[name][1](ctor);
        delete waiting[name];
      }
    },
    get: function get(name) {
      need('get', 1, arguments.length);
      var def = defs[String(name)];
      return def ? def.ctor : undefined;
    },
    getName: function getName(ctor) {
      need('getName', 1, arguments.length);
      var def = ctors.get(ctor);
      return def ? def.name : null;
    },
    whenDefined: function whenDefined(name) {
      name = String(name);
      if (!valid(name)) {
        return Promise.reject(err('SyntaxError', at + "whenDefined: '" + name + "' is not a valid custom element name"));
      }
      if (defs[name]) return Promise.resolve(defs[name].ctor);
      if (!waiting[name]) {
        var resolve;
        var p = new Promise(function (r) {
          resolve = r;
        });
        waiting[name] = [p, resolve];
      }
      return waiting[name][0];
    },
    upgrade: function upgrade_(root) {
      need('upgrade', 1, arguments.length);
      if (!(root instanceof g.Node)) throw new TypeError(at + 'upgrade: Argument 1 does not implement interface Node.');
      ce.scope(function () {
        ce.each(slots(root).id, function (w) {
          ce.enqueue(w, ce.UPGRADE);
        });
      });
    },
  });
  var registry = Object.create(CER.prototype);
  Object.defineProperty(g, 'customElements', {
    get: brand(function () {
      return registry;
    }, 'get customElements'),
    set: brand(function (v) {
      Object.defineProperty(g, 'customElements', { value: v, writable: true, enumerable: true, configurable: true });
    }, 'set customElements'),
    enumerable: true,
    configurable: true,
  });
  Object.defineProperty(g, '__frot_html_construct', { value: brand(construct, '__frot_html_construct'), configurable: true });
  // createElement with a defined name runs the constructor now, as the
  // element is made; `{is}` names a customized built-in.
  var create = g.Document.prototype.createElement;
  // A constructor that throws, or answers anything but the element it was
  // built for, leaves an HTMLUnknownElement of the name, and the error is
  // reported (measured).
  g.Document.prototype.createElement = brand(function createElement(tag) {
    var options = arguments[1];
    var local = String(tag).toLowerCase();
    var is = options && typeof options === 'object' && options.is !== undefined ? String(options.is) : null;
    if (valid(local)) is = null;
    var def = ce.definition(local, is);
    if (!def) {
      var el = create.call(this, tag);
      if (is !== null) ce.isValues.set(slots(el).id, is);
      return el;
    }
    try {
      var made = new def.ctor();
      if (made instanceof g.Node && slots(made).custom === def) return made;
      throw err('NotSupportedError', 'Operation is not supported');
    } catch (e) {
      g.reportError(e);
      var w = wrap(ce.raw.create_element(local));
      Object.setPrototypeOf(w, g.HTMLUnknownElement.prototype);
      slots(w).failed = true;
      return w;
    }
  }, 'createElement');
})(globalThis);
