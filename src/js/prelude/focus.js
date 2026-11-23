// Focus and the top layer (js.md §7; bl-3a36, bl-bc6d). The state lives in
// the arena, not here: `document.activeElement`, `:focus`, `:modal` and
// `:popover-open` all read the one answer the host keeps (dom/live.rs), so a
// script's `el.focus()` is what the cascade sees and an untouched page's
// `activeElement` is its `autofocus` element, as in a browser.
//
// The field trial's React TodoMVC deployment dies without focus(): React's
// commit phase runs `autoFocus && stateNode.focus()` on mount, the throw is
// caught by captureCommitPhaseError, and with no error boundary the whole root
// unmounts. Page-driven, never host-driven: frot dispatches only
// DOMContentLoaded/load (js.md §11 — no interaction), so nothing here moves
// focus by itself; a page script calling focus() is the page acting. What may
// take focus is the host's rule (css/focus.rs `focusable`); focus() on anything
// else is the browser's silent no-op. Moving focus fires `blur` on the element
// losing it and `focus` on the one gaining it. Loads after events.js.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;

  function emit(el, type) {
    el.dispatchEvent(new g.Event(type));
  }
  function id(el) {
    return slots(el).id;
  }

  g.__frot_ifaceattrs(g.Document.prototype, {
    activeElement: function () {
      return wrap(g.__frot_focused()) || this.body;
    },
  });
  // Popovers: shown by script only, hidden by the UA rule until then.
  function popover(el, open) {
    if (!el.hasAttribute('popover')) {
      throw g.__frot_domerror('NotSupportedError', 'Element does not have the popover attribute');
    }
    return g.__frot_popover(id(el), open);
  }
  // Showing one focuses its first `autofocus` descendant that can take focus
  // (Chrome 158), and nothing else.
  g.__frot_ifaceops(g.HTMLElement.prototype, {
    showPopover: function showPopover() {
      if (popover(this, null)) return;
      popover(this, true);
      Array.prototype.slice.call(this.querySelectorAll('[autofocus]')).find(function (el) {
        return g.__frot_focus(id(el));
      });
    },
    hidePopover: function hidePopover() {
      popover(this, false);
    },
    togglePopover: function togglePopover(force) {
      var open = force === undefined ? !popover(this, null) : !!force;
      if (open) this.showPopover();
      else this.hidePopover();
      return open;
    },
  });

  // On HTMLElement.prototype and, separately, SVGElement.prototype (measured).
  [g.HTMLElement.prototype, g.SVGElement.prototype].forEach(function (proto) {
    g.__frot_ifaceops(proto, {
      focus: function focus() {
        var prev = g.__frot_focused();
        if (prev === id(this) || !g.__frot_focus(id(this))) return;
        if (prev !== null && prev !== undefined) emit(wrap(prev), 'blur'); // the loser first
        emit(this, 'focus');
      },
      blur: function blur() {
        if (g.__frot_focused() !== id(this)) return;
        g.__frot_blur();
        emit(this, 'blur');
      },
    });
  });

  // Dialogs: show() opens, showModal() opens modally, close() closes, hands
  // focus back to where it was before the dialog took it, and queues `close`.
  // All three keep the `open` attribute, as browsers do.
  // Opening runs the dialog focusing steps, measured in Chrome 158: the first
  // `autofocus` descendant that can take focus, else the first descendant
  // that can, else the dialog itself.
  function focusDialog(dialog) {
    slots(dialog).returnFocus = g.__frot_focused();
    var inside = Array.prototype.slice.call(dialog.querySelectorAll('*'));
    var auto = inside.filter(function (el) {
      return el.hasAttribute('autofocus');
    });
    var target = auto.concat(inside).find(function (el) {
      return g.__frot_focus(id(el));
    });
    if (!target) g.__frot_focus(id(dialog), true);
  }
  g.__frot_ifaceops(g.HTMLDialogElement.prototype, {
    show: function show() {
      if (this.hasAttribute('open')) return;
      this.setAttribute('open', '');
      focusDialog(this);
    },
    showModal: function showModal() {
      if (this.hasAttribute('open')) return;
      this.setAttribute('open', '');
      g.__frot_modal(id(this), true);
      focusDialog(this);
    },
    close: function close(value) {
      if (!this.hasAttribute('open')) return;
      if (value !== undefined) this.returnValue = String(value);
      this.removeAttribute('open');
      g.__frot_modal(id(this), false);
      // Focus inside the dialog goes back where it was before it opened.
      var back = slots(this).returnFocus;
      if (this === g.document.activeElement || this.contains(g.document.activeElement)) {
        if (back === null || back === undefined || !g.__frot_focus(back)) g.__frot_blur();
      }
      var self = this;
      g.setTimeout(function () {
        emit(self, 'close');
      }, 0);
    },
  });
  g.__frot_tagdef('Dialog', 'open', {
    get: function () {
      return this.hasAttribute('open');
    },
    set: function (v) {
      if (v) this.setAttribute('open', '');
      else this.removeAttribute('open');
    },
  });
})(globalThis);
