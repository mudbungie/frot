// MutationObserver (js.md §7, bl-07ab) — GENUINE observation at the one
// mutation seam. Every tree/attribute/text mutation reaches the arena through
// exactly five prelude-visible syscalls (§2/§3: __frot_insert_child,
// __frot_detach, __frot_set_attr, __frot_remove_attr, __frot_set_text), so one
// wrapper set over those five yields real MutationRecords — childList /
// attributes / characterData, oldValue, subtree matching by parent-chain walk
// (O(depth) per mutation, never a subtree scan) — delivered as microtasks on
// the engine job queue (§5), the spec's own timing. No new Rust surface: the
// raw syscalls are captured here and the wrapped names re-published in their
// place, so every DOM API in the prelude is covered without per-API hooks.
// All state (registrations, record queues) is realm-local per invocation.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var raw = {
    insert: g.__frot_insert_child,
    detach: g.__frot_detach,
    setAttr: g.__frot_set_attr,
    removeAttr: g.__frot_remove_attr,
    setText: g.__frot_set_text,
    parent: g.__frot_parent,
    children: g.__frot_children,
    attr: g.__frot_attr,
    text: g.__frot_text,
    kind: g.__frot_kind,
    roots: g.__frot_roots,
  };

  // Registrations: target key -> [{mo, options}]. Keys are arena NodeIds; the
  // document (not an arena node) registers under the 'doc' sentinel and is
  // matched for any CONNECTED node's mutation via subtree — a childList
  // mutation's target is always a real parent node, so document-as-target
  // never occurs and the sentinel needs no walk entry of its own. `active` is
  // the total registration count: the zero-cost fast path for the (common)
  // page that never constructs an observer.
  var registry = Object.create(null);
  var active = 0;
  var pending = []; // observers with queued records, in first-queue order
  var scheduled = false;

  // The notify microtask (spec: "queue a mutation observer microtask", once).
  // The flag clears BEFORE callbacks run, so a callback that itself mutates
  // the DOM schedules a fresh round — re-entrancy is a new batch, exactly the
  // browser semantic, and it terminates under the §5 compute budget.
  function notify() {
    scheduled = false;
    var batch = pending;
    pending = [];
    for (var i = 0; i < batch.length; i++) {
      var mo = batch[i];
      mo._queued = false;
      var records = mo.takeRecords();
      if (!records.length) continue;
      try {
        mo._cb.call(mo, records, mo);
      } catch (e) {
        // A throwing observer callback is an uncaught error in a browser:
        // route it through the §10 reportError channel (counted, and its
        // message captured, unless the page's own error handling suppresses).
        g.reportError(e);
      }
    }
  }

  function enqueue(mo, record) {
    mo._records.push(record);
    if (!mo._queued) {
      mo._queued = true;
      pending.push(mo);
    }
    if (!scheduled) {
      scheduled = true;
      queueMicrotask(notify);
    }
  }

  // Collect the observers interested in a mutation of `type` at node `id`:
  // walk the parent chain once (the target itself, then ancestors needing
  // subtree), then the document sentinel if the chain topped out at a
  // connected root. Dedupes by observer, OR-folding whether any matching
  // registration wants oldValue (spec: one record per interested observer).
  function interested(id, type, attrName) {
    var out = [];
    var top = id;
    var isTarget = true;
    for (var n = id; n !== null && n !== undefined; n = raw.parent(n)) {
      top = n;
      var regs = registry[n];
      if (regs) collect(regs, isTarget, type, attrName, out);
      isTarget = false;
    }
    var doc = registry.doc;
    if (doc && doc.length && raw.roots().indexOf(top) >= 0) {
      collect(doc, false, type, attrName, out);
    }
    return out;
  }

  function collect(regs, isTarget, type, attrName, out) {
    for (var i = 0; i < regs.length; i++) {
      var o = regs[i].options;
      if (!isTarget && !o.subtree) continue;
      var old = false;
      if (type === 'childList') {
        if (!o.childList) continue;
      } else if (type === 'attributes') {
        if (!o.attributes || (o.filter && o.filter.indexOf(attrName) < 0)) continue;
        old = o.attrOld;
      } else {
        if (!o.characterData) continue;
        old = o.charOld;
      }
      var mo = regs[i].mo;
      var seen = null;
      for (var j = 0; j < out.length; j++) {
        if (out[j].mo === mo) {
          seen = out[j];
          break;
        }
      }
      if (seen) seen.old = seen.old || old;
      else out.push({ mo: mo, old: old });
    }
  }

  // --- MutationRecord: Firefox-shaped ([object MutationRecord], Illegal
  // constructor), fields as non-enumerable own props over the branded proto.
  var MutationRecord = g.__frot_iface('MutationRecord', {});
  function def(o, k, v) {
    Object.defineProperty(o, k, { value: v, configurable: true, writable: true });
  }
  function mkRecord(type, targetId) {
    var r = Object.create(MutationRecord.prototype);
    def(r, 'type', type);
    def(r, 'target', new g.Node(targetId));
    def(r, 'addedNodes', g.__frot_nodelist([]));
    def(r, 'removedNodes', g.__frot_nodelist([]));
    def(r, 'previousSibling', null);
    def(r, 'nextSibling', null);
    def(r, 'attributeName', null);
    def(r, 'attributeNamespace', null);
    def(r, 'oldValue', null);
    return r;
  }
  // A childList record for `child` under `parent`; for a removal this runs
  // BEFORE the arena detach, so the child's true siblings are still readable.
  function childRecord(parentId, childId, added) {
    var r = mkRecord('childList', parentId);
    var kids = raw.children(parentId);
    var at = kids.indexOf(childId);
    def(r, added ? 'addedNodes' : 'removedNodes', g.__frot_nodelist([childId]));
    def(r, 'previousSibling', at > 0 ? new g.Node(kids[at - 1]) : null);
    def(r, 'nextSibling', at >= 0 && at + 1 < kids.length ? new g.Node(kids[at + 1]) : null);
    return r;
  }

  // --- The five-syscall seam. Each wrapper: zero-observer fast path, find the
  // interested set, capture any oldValue BEFORE the raw mutation, mutate, queue.
  function seam(name, fn) {
    g[name] = brand(fn, name);
  }
  seam('__frot_set_attr', function (id, name, value) {
    if (active === 0 || raw.kind(id) !== 'element') return raw.setAttr(id, name, value);
    var an = String(name).toLowerCase(); // the arena stores lowercased names
    var list = interested(id, 'attributes', an);
    var old = null;
    if (list.length) {
      var v = raw.attr(id, an);
      old = v == null ? null : v;
    }
    raw.setAttr(id, name, value);
    for (var i = 0; i < list.length; i++) {
      var r = mkRecord('attributes', id);
      def(r, 'attributeName', an);
      if (list[i].old) def(r, 'oldValue', old);
      enqueue(list[i].mo, r);
    }
  });
  seam('__frot_remove_attr', function (id, name) {
    if (active === 0 || raw.kind(id) !== 'element') return raw.removeAttr(id, name);
    var an = String(name).toLowerCase();
    var list = interested(id, 'attributes', an);
    var old = list.length ? raw.attr(id, an) : null;
    raw.removeAttr(id, name);
    if (old == null) return; // removing an absent attribute is no mutation
    for (var i = 0; i < list.length; i++) {
      var r = mkRecord('attributes', id);
      def(r, 'attributeName', an);
      if (list[i].old) def(r, 'oldValue', old);
      enqueue(list[i].mo, r);
    }
  });
  seam('__frot_set_text', function (id, text) {
    if (active === 0 || raw.kind(id) !== 'text') return raw.setText(id, text);
    var list = interested(id, 'characterData', null);
    var old = list.length ? raw.text(id) : null;
    raw.setText(id, text);
    for (var i = 0; i < list.length; i++) {
      var r = mkRecord('characterData', id);
      if (list[i].old) def(r, 'oldValue', old);
      enqueue(list[i].mo, r);
    }
  });
  seam('__frot_insert_child', function (parent, child, before) {
    if (active === 0) return raw.insert(parent, child, before);
    // The arena auto-unlinks on insert (§2), so inserting an already-parented
    // node is honestly a MOVE: a removal record for the old parent (siblings
    // captured pre-unlink), then the addition — the spec's two records.
    var oldParent = raw.parent(child);
    var rlist = null;
    var removal = null;
    if (oldParent !== null && oldParent !== undefined) {
      rlist = interested(oldParent, 'childList', null);
      if (rlist.length) removal = childRecord(oldParent, child, false);
    }
    raw.insert(parent, child, before);
    if (removal) for (var i = 0; i < rlist.length; i++) enqueue(rlist[i].mo, removal);
    var alist = interested(parent, 'childList', null);
    if (alist.length) {
      var added = childRecord(parent, child, true);
      for (var j = 0; j < alist.length; j++) enqueue(alist[j].mo, added);
    }
  });
  seam('__frot_detach', function (id) {
    if (active === 0) return raw.detach(id);
    var parent = raw.parent(id);
    if (parent === null || parent === undefined) return raw.detach(id); // already detached: no mutation
    var list = interested(parent, 'childList', null);
    var rec = list.length ? childRecord(parent, id, false) : null;
    raw.detach(id);
    if (rec) for (var i = 0; i < list.length; i++) enqueue(list[i].mo, rec);
  });

  // --- The registration seam for observer2.js's MutationObserver interface
  // (the same non-enumerable-global pattern as __frot_elems; split files keep
  // each under the 300-line source cap). register() replaces an existing
  // (mo, key) registration — spec: re-observe on the same target replaces — or
  // adds one; unregister() removes all of mo's registrations for `keys`.
  Object.defineProperty(g, '__frot_mo_hook', {
    value: {
      register: function (mo, key, options) {
        var regs = registry[key] || (registry[key] = []);
        for (var i = 0; i < regs.length; i++) {
          if (regs[i].mo === mo) {
            regs[i].options = options;
            return false; // replaced, not a new registration
          }
        }
        regs.push({ mo: mo, options: options });
        active++;
        return true;
      },
      unregister: function (mo, keys) {
        for (var i = 0; i < keys.length; i++) {
          var regs = registry[keys[i]];
          for (var j = regs.length - 1; j >= 0; j--) {
            if (regs[j].mo === mo) {
              regs.splice(j, 1);
              active--;
            }
          }
          if (!regs.length) delete registry[keys[i]];
        }
      },
    },
    configurable: true,
    writable: true,
  });
})(globalThis);
