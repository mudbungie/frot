// Events: the ONE listener registry and the EventTarget interface over it
// (bl-e81b, split out of loop.js by bl-6438; js.md §4.4/§11, identity.md §10).
// DOMContentLoaded/load are the only events frot's host ever fires (§11) —
// everything else registered here runs only if the page dispatches it itself, so
// this file is a registry and a shape, not a scheduler: WHEN the host fires lives
// in loop.js, which dispatches through the `__frot_dispatch` helper exported at
// the end. It moved out of loop.js when EventTarget stopped being a constructor
// that stamped three methods onto each instance and became a real INTERFACE with
// its methods on the prototype — the shape Firefox has, and the one every
// masqueraded event target (Worker, SharedWorker, MessagePort, PermissionStatus,
// Notification, IDBRequest) now inherits instead of flattening (`bl-6438`,
// identity.md §3.14). Runs after brand.js (branding) and dom.js (Node/document),
// and BEFORE loop.js, permissions.js, worker.js and idb.js, which all reach it.
(function (g) {
  'use strict';

  // The one registry. Listeners key by TARGET so the stateless Node wrappers
  // (fresh per syscall) never hold them, and so window/document/nodes/fragments/
  // XHR/every masqueraded interface share one implementation.
  var reg = Object.create(null);
  function bucket(key, type) {
    var t = reg[key] || (reg[key] = Object.create(null));
    return t[type] || (t[type] = []);
  }
  function add(key, type, fn) {
    if (typeof fn !== 'function') return;
    var b = bucket(key, type);
    if (b.indexOf(fn) < 0) b.push(fn);
  }
  function remove(key, type, fn) {
    var b = bucket(key, type);
    var i = b.indexOf(fn);
    if (i >= 0) b.splice(i, 1);
  }
  function fire(key, ev, target) {
    var b = bucket(key, ev.type).slice();
    var errs = 0;
    for (var i = 0; i < b.length; i++)
      try {
        b[i].call(target, ev);
      } catch (e) {
        errs++;
      }
    return errs;
  }
  // Which registry bucket an EventTarget's listeners live in. Lazy, and held in a
  // WeakMap rather than on the object, so the key is never a property a page can
  // walk. `window` and `document` are seeded with the fixed keys the lifecycle
  // dispatch below fires on.
  var etSeq = 0;
  var etKeys = new WeakMap();
  function etKey(o) {
    var k = etKeys.get(o);
    if (k === undefined) {
      k = 'et' + ++etSeq;
      etKeys.set(o, k);
    }
    return k;
  }

  g.Event = function (type, init) {
    this.type = String(type);
    this.bubbles = !!(init && init.bubbles);
    this.defaultPrevented = false;
    this.target = null;
  };
  g.Event.prototype.preventDefault = function () {
    this.defaultPrevented = true;
  };
  g.Event.prototype.stopPropagation = function () {};
  g.CustomEvent = function (type, init) {
    g.Event.call(this, type, init);
    this.detail = init ? init.detail : null;
  };
  g.CustomEvent.prototype = Object.create(g.Event.prototype);

  // EventTarget (bl-e81b): Firefox's base interface, constructable since FF 59
  // — framer-motion resolves animation targets with `t instanceof EventTarget`,
  // a ReferenceError while the name is absent. Instances ride the one registry;
  // `instanceof` matches by the trio's shape, so every event-bearing surface
  // (window, document, nodes, fragments, XHR, …) answers true without a class
  // hierarchy — the elem.js hasInstance pattern.
  //
  // The trio lives ON THE PROTOTYPE, where Firefox's does: measured on 153.0esr
  // (`bl-6438`, identity.md §3.14) `Object.getPrototypeOf(Worker.prototype)
  // .constructor.name === 'EventTarget'`, and `Worker.prototype` owns six
  // properties, none of them a listener method. So this is THE EventTarget
  // interface every masqueraded one inherits (Worker, SharedWorker, MessagePort,
  // PermissionStatus, Notification, IDBRequest) instead of each flattening three
  // copies of its own onto itself.
  g.EventTarget = function EventTarget() {};
  g.EventTarget.prototype.addEventListener = function (type, fn) {
    add(etKey(this), String(type), fn);
  };
  g.EventTarget.prototype.removeEventListener = function (type, fn) {
    remove(etKey(this), String(type), fn);
  };
  g.EventTarget.prototype.dispatchEvent = function (ev) {
    fire(etKey(this), ev, this);
    return !(ev && ev.defaultPrevented);
  };
  Object.defineProperty(g.EventTarget, Symbol.hasInstance, {
    value: function (o) {
      return !!o && (typeof o === 'object' || typeof o === 'function') && typeof o.addEventListener === 'function';
    },
  });

  // window and document are event targets with FIXED registry keys (the lifecycle
  // dispatch below fires on those keys), so they take the same three functions
  // rather than copies. Branding here, at the definition, is what makes them read
  // native everywhere they are reached — including through every prototype that
  // now inherits them (nativebrand.js's final sweep only walks own properties).
  ['addEventListener', 'removeEventListener', 'dispatchEvent'].forEach(function (k) {
    g.__frot_brand(g.EventTarget.prototype[k], k);
    g[k] = g.EventTarget.prototype[k];
    g.document[k] = g.EventTarget.prototype[k];
  });
  etKeys.set(g, 'window');
  etKeys.set(g.document, 'document');
  g.Node.prototype.addEventListener = function (type, fn) {
    add('n' + this._id, String(type), fn);
  };
  g.Node.prototype.removeEventListener = function (type, fn) {
    remove('n' + this._id, String(type), fn);
  };
  g.Node.prototype.dispatchEvent = function (ev) {
    fire('n' + this._id, ev, this);
    return !(ev && ev.defaultPrevented);
  };

  // The registry's dispatch, for the host lifecycle in loop.js: the only caller
  // outside this file, and the reason `fire` is not simply private. Non-enumerable
  // like every other `__frot_*` helper, and branded so its source never shows.
  g.__frot_brand(fire, '__frot_dispatch');
  Object.defineProperty(g, '__frot_dispatch', {
    value: fire,
    configurable: true,
    writable: true,
  });
})(globalThis);
