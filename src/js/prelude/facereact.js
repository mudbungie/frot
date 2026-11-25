// The reactions of a form-associated custom element (js.md §15.6): its
// formAssociatedCallback when its form owner changes and its
// formDisabledCallback when its disabled state does, queued beside the custom
// element reactions (cereact.js) in Firefox 153.0esr's order
// (js/tests/internals/). The owner and the disabled state are queries of the
// arena (`dom::form`, `:disabled`); what is kept per element is only the last
// answer each callback reported, which is the form owner `internals.form`
// reads. A realm with no form-associated definition pays one test per
// mutation.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var wrap = g.__frot_wrap;
  var brand = g.__frot_brand;
  var ce = g.__frot_ce;

  // Each form-associated element in the subtree at `id`.
  function faces(id, fn) {
    ce.each(id, function (w) {
      if (slots(w).custom && slots(w).custom.face) fn(w);
    });
  }
  function owner(w, form) {
    var st = slots(w);
    var was = st.owner === undefined ? null : st.owner;
    if (form === was) return;
    st.owner = form;
    ce.enqueue(w, ['formAssociatedCallback', [form === null ? null : wrap(form)]]);
  }
  function reassociate(w) {
    var form = g.__frot_form_owner(slots(w).id);
    owner(w, form == null ? null : form);
  }
  function disabled(w) {
    var st = slots(w);
    var now = g.__frot_matches(st.id, ':disabled');
    if (now === !!st.disabled) return;
    st.disabled = now;
    ce.enqueue(w, ['formDisabledCallback', [now]]);
  }
  function settle(id) {
    faces(id, function (w) {
      reassociate(w);
      disabled(w);
    });
  }
  // The subtree at `id` leaves its parent: each element's owner becomes its
  // nearest form ancestor inside the subtree (a detached element's `form`
  // attribute names nothing).
  function leaving(id) {
    faces(id, function (w) {
      var form = null;
      for (var n = slots(w).id; n !== id && form === null;) {
        n = g.__frot_parent(n);
        if (g.__frot_tag(n) === 'form') form = n;
      }
      owner(w, form);
    });
  }

  // A newly made form-associated element (cereact.js `made`).
  ce.face = function (w) {
    g.__frot_set_face(slots(w).id);
    reassociate(w);
    disabled(w);
  };

  function seam(name, around) {
    var f = g['__frot_' + name];
    g['__frot_' + name] = brand(function () {
      if (!ce.faces) return f.apply(this, arguments);
      return ce.within(around, this, [f, arguments]);
    }, '__frot_' + name);
  }
  // A move is a removal, then an insertion: the removal's callback comes
  // before the insertion's connectedCallback (measured).
  seam('insert_child', function (f, a) {
    var child = a[1];
    var whole = g.__frot_kind(child) === 'document-fragment';
    var moved = whole ? g.__frot_children(child) : [child];
    if (!whole && g.__frot_parent(child) != null) leaving(child);
    var why = f.apply(null, a);
    if (!why) moved.forEach(settle);
    return why;
  });
  seam('detach', function (f, a) {
    leaving(a[0]);
    f.apply(null, a);
    settle(a[0]);
  });
  function attr(f, a) {
    f.apply(null, a);
    var name = String(a[1]).toLowerCase();
    if (name === 'form' || name === 'disabled') settle(a[0]);
  }
  seam('set_attr', attr);
  seam('remove_attr', attr);
})(globalThis);
