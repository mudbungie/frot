// What a <script> element adds (js.md §4, bl-a86b): `text`, and the `load` or
// `error` the host fires at an external one.
//
// An external <script>'s `load` or `error`. When the host
// has run an external script — or failed to fetch it — it fires one of the two
// at the element, as Firefox does for every external script, parsed or
// inserted: `load` after it ran (even if it threw), `error` when its fetch
// failed. A handler's throw is reported like any listener's (events.js).
(function (g) {
  'use strict';
  // `text` is the script's source: its text, and setting it replaces the
  // children with one text node — which can make an inserted, empty script
  // runnable, exactly as `textContent` does.
  g.__frot_tagdef('Script', 'text', {
    get: function () {
      return this.textContent;
    },
    set: function (v) {
      this.textContent = String(v);
    },
  });
  Object.defineProperty(g, '__frot_script_event', {
    configurable: true,
    value: function (id, type) {
      var el = g.__frot_wrap(id);
      var ev = new g.Event(type);
      g.__frot_slots(ev).trusted = true; // the browser raised it, not the page
      g.__frot_dispatch(el, ev); // its `on<type>` handler included
      return 0;
    },
  });
})(globalThis);
