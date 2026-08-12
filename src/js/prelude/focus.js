// Focus management (js.md §7, bl-3a36) — one document-level fact,
// `document.activeElement`, that `focus()`/`blur()` move. The field trial's
// React TodoMVC deployment dies without it: React's commit phase runs
// `autoFocus && stateNode.focus()` on mount, the throw is caught by
// captureCommitPhaseError, and with no error boundary the whole root unmounts —
// a settled run, one "not a function" report, and an empty shell.
//
// Page-driven, never host-driven: frot dispatches only DOMContentLoaded/load
// (js.md §11 — no interaction), so nothing here focuses anything by itself; a page
// script calling focus() is the page acting, exactly like dispatchEvent. The
// state is one wrapper handle (wrappers cache no arena state, §2). Focusability
// follows Firefox: form controls (input except type=hidden, textarea, select,
// button) unless disabled, a/area with href, and anything carrying tabindex or
// contenteditable; focus() on anything else is the browser's silent no-op.
// Moving focus fires `blur` on the element losing it and `focus` on the one
// gaining it, through the same listener registry as every page dispatch. Loads
// after events.js (needs Node.dispatchEvent).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var proto = g.Node.prototype;
  var active = null; // the focused element's wrapper, or null (=> body)

  function focusable(el) {
    if (el.nodeType !== 1 || el.hasAttribute('disabled')) return false;
    var t = el.tagName;
    if (t === 'INPUT') return el.getAttribute('type') !== 'hidden';
    if (t === 'TEXTAREA' || t === 'SELECT' || t === 'BUTTON') return true;
    if ((t === 'A' || t === 'AREA') && el.hasAttribute('href')) return true;
    return el.hasAttribute('tabindex') || el.hasAttribute('contenteditable');
  }
  function emit(el, type) {
    var ev = new g.Event(type);
    ev.target = el;
    el.dispatchEvent(ev);
  }

  Object.defineProperty(g.document, 'activeElement', {
    configurable: true,
    get: function () {
      return active || g.document.body;
    },
  });
  proto.focus = function focus() {
    if (!focusable(this) || (active && slots(active).id === slots(this).id)) return;
    var prev = active;
    active = this;
    if (prev) emit(prev, 'blur'); // blur the loser first, as browsers order it
    emit(this, 'focus');
  };
  proto.blur = function blur() {
    if (!active || slots(active).id !== slots(this).id) return;
    active = null;
    emit(this, 'blur');
  };
})(globalThis);
