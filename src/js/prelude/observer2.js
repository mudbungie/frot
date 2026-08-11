// The MutationObserver interface (js.md §7, bl-07ab) — the web-facing half of
// observer.js's mutation-syscall seam, split out to keep both files under the
// 300-line source cap. Loads after observer.js (consumes its non-enumerable
// __frot_mo_hook registration seam) and holds no record/queue mechanics of its
// own: observe()/disconnect() edit the seam's registry, takeRecords() drains
// the per-instance queue observer.js fills.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var hook = g.__frot_mo_hook;
  function def(o, k, v) {
    Object.defineProperty(o, k, { value: v, configurable: true, writable: true });
  }

  // Registration key: the document observes under the 'doc' sentinel (matched
  // for connected subtrees); a Node wrapper under its NodeId. A fake node
  // (elem2.js comment/fragment, _id -1) registers and simply never matches.
  function keyOf(target) {
    if (target === g.document) return 'doc';
    if (target && typeof target._id === 'number') return target._id;
    throw new TypeError('MutationObserver.observe: Argument 1 does not implement interface Node.');
  }
  // Spec §4.3.1 option validation: *OldValue/attributeFilter imply their type
  // when it is omitted, contradict it when it is explicitly false, and at
  // least one of the three types must end up requested.
  function normalize(opts) {
    opts = opts || {};
    var filter = opts.attributeFilter === undefined ? null
      : Array.prototype.map.call(opts.attributeFilter, function (s) {
        return String(s).toLowerCase(); // the arena stores lowercased names
      });
    var o = {
      childList: !!opts.childList,
      subtree: !!opts.subtree,
      attributes: opts.attributes === undefined
        ? opts.attributeOldValue !== undefined || filter !== null
        : !!opts.attributes,
      attrOld: !!opts.attributeOldValue,
      filter: filter,
      characterData: opts.characterData === undefined
        ? opts.characterDataOldValue !== undefined
        : !!opts.characterData,
      charOld: !!opts.characterDataOldValue,
    };
    if (!o.childList && !o.attributes && !o.characterData) {
      throw new TypeError("MutationObserver.observe: One of 'childList', 'attributes', 'characterData' must be true");
    }
    if ((o.attrOld || o.filter) && !o.attributes) {
      throw new TypeError('MutationObserver.observe: attributes option must be true');
    }
    if (o.charOld && !o.characterData) {
      throw new TypeError('MutationObserver.observe: characterData option must be true');
    }
    return o;
  }

  var holder = {
    MutationObserver: function (cb) {
      if (!(this instanceof holder.MutationObserver)) {
        throw new TypeError("Constructor MutationObserver requires 'new'");
      }
      if (typeof cb !== 'function') {
        throw new TypeError('MutationObserver constructor: Argument 1 is not callable.');
      }
      def(this, '_cb', cb);
      def(this, '_records', []);
      def(this, '_queued', false);
      def(this, '_keys', []);
    },
  };
  var Ctor = brand(holder.MutationObserver, 'MutationObserver');
  var proto = Ctor.prototype;
  Object.defineProperty(proto, Symbol.toStringTag, { value: 'MutationObserver', configurable: true });
  proto.observe = brand(function observe(target, options) {
    var key = keyOf(target);
    var o = normalize(options);
    if (hook.register(this, key, o)) this._keys.push(key);
    return undefined;
  }, 'observe');
  proto.disconnect = brand(function disconnect() {
    var keys = this._keys;
    this._keys = [];
    hook.unregister(this, keys);
    this._records = []; // spec: disconnect empties the record queue
    return undefined;
  }, 'disconnect');
  proto.takeRecords = brand(function takeRecords() {
    var records = this._records;
    this._records = [];
    return records;
  }, 'takeRecords');
  Object.defineProperty(g, 'MutationObserver', { value: Ctor, configurable: true, writable: true });
})(globalThis);
