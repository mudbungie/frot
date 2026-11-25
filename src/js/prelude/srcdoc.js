// `<iframe srcdoc>` (bl-7b86; js.md §14), as Firefox 153.0esr loads it
// (src/js/tests/srcdoc/firefox153.json). Right after insertion the frame shows
// its initial about:blank; a task later — after the zero-delay timers queued
// with it — it navigates to about:srcdoc: the document is replaced by the
// parsed srcdoc, its scripts run in order in the frame's global (inserted,
// so through js.md §4.7's gate),
// DOMContentLoaded and load fire in the frame, then `load` at the iframe.
// The window stays the same object, as the binary keeps it for a same-origin
// navigation away from the initial about:blank; the document object stays too
// (Firefox's is new — declared). Setting srcdoc navigates again; removing it
// from a frame with no `src` navigates back to about:blank. A `src` frame
// still never navigates (§14.2).
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;

  // This window, navigated in place to `html` at `url`.
  function navigate(html, url) {
    var doc = g.document, id = slots(doc).id;
    g.__frot_relocate(url);
    slots(doc).readyState = 'loading';
    g.__frot_children(id).forEach(function (kid) {
      g.__frot_detach(kid);
    });
    // Inserting the parsed nodes runs their scripts, in tree order, through
    // the one gate every inserted script takes (js.md §4.7).
    var parsed = g.__frot_parse_document(html);
    g.__frot_children(parsed).forEach(function (kid) {
      g.__frot_insert_child(id, kid, null);
    });
    g.__frot_fire('DOMContentLoaded');
    g.__frot_fire('load');
  }
  Object.defineProperty(g, '__frot_navigate', { value: brand(navigate, '__frot_navigate'), configurable: true });
  // `srcdoc` reflects its attribute, as every DOM-string attribute does.
  g.__frot_tagdef('IFrame', 'srcdoc', g.__frot_reflect('srcdoc'));

  // An iframe of this document whose srcdoc (or its removal) asks for a
  // navigation: a task that queues the navigation as one more task.
  function load(el) {
    g.__frot_task(function () {
      g.__frot_task(function () {
        var sd = el.getAttribute('srcdoc');
        if (!el.isConnected || (sd === null && el.hasAttribute('src'))) return;
        el.contentWindow.__frot_navigate(sd === null ? '' : sd, sd === null ? 'about:blank' : 'about:srcdoc');
        var ev = new g.Event('load');
        slots(ev).trusted = true;
        g.__frot_dispatch(el, ev);
      });
    });
  }
  function loadAll(id) {
    if (g.__frot_tag(id) === 'iframe' && g.__frot_attr(id, 'srcdoc') != null) load(g.__frot_wrap(id));
    g.__frot_query(id, 'iframe[srcdoc]').forEach(function (f) {
      load(g.__frot_wrap(f));
    });
  }
  var raw = { insert: g.__frot_insert_child, set: g.__frot_set_attr, remove: g.__frot_remove_attr };
  function seam(name, fn) {
    Object.defineProperty(g, name, { value: brand(fn, name), configurable: true, writable: true });
  }
  function srcdocOf(id, name) {
    return g.__frot_tag(id) === 'iframe' && String(name).toLowerCase() === 'srcdoc' && g.__frot_connected(id);
  }
  seam('__frot_insert_child', function (parent, child, before) {
    var kids = g.__frot_kind(child) === 'document-fragment' ? g.__frot_children(child) : [child];
    var why = raw.insert(parent, child, before);
    if (!why && g.__frot_connected(parent)) kids.forEach(loadAll);
    return why;
  });
  seam('__frot_set_attr', function (id, name, value) {
    raw.set(id, name, value);
    if (srcdocOf(id, name)) load(g.__frot_wrap(id));
  });
  seam('__frot_remove_attr', function (id, name) {
    var had = g.__frot_attr(id, 'srcdoc') != null;
    raw.remove(id, name);
    if (had && srcdocOf(id, name)) load(g.__frot_wrap(id));
  });
  // The parsed document's srcdoc frames load like inserted ones.
  loadAll(slots(g.document).id);
})(globalThis);
