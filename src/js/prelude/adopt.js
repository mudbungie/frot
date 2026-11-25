// Cross-document adoption (bl-2aaa; js.md §14.4). Every window's document is
// its own arena, so a node made in a frame and inserted into its parent is a
// wrapper whose id means nothing here. Firefox 153.0esr adopts it (measured,
// src/js/tests/frame/firefox153.json): the same object, now in this document —
// removed from its old parent, `ownerDocument` this one, its descendants and
// its listeners with it. Here adoption is a copy of the subtree into this
// arena, read through the other window's syscalls and written through this
// one's, and then every wrapper the other window had for the subtree is bound
// to its copy — same objects, so a script's references keep working — with its
// listeners moved to this window's registry. One declared difference: an
// adopted wrapper takes THIS window's prototype (Firefox keeps the frame's, so
// `p instanceof FW.HTMLParagraphElement` there and `p instanceof
// HTMLParagraphElement` here), because a frame's methods can only reach the
// frame's own arena. importNode across documents copies and binds nothing.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;

  // The window whose arena `node`'s id belongs to, when it is not this one.
  function home(node) {
    if (node === null || typeof node !== 'object' || typeof slots(node).id === 'number') return null;
    var w = node.ownerDocument && node.ownerDocument.defaultView;
    return w && w !== g && typeof w.__frot_slots === 'function' && typeof w.__frot_slots(node).id === 'number'
      ? w
      : null;
  }
  // Copy `from`'s node `fid` (deep: its subtree) into this arena; `pairs`
  // collects [foreign id, new id] in tree order.
  function copy(from, fid, deep, pairs) {
    var kind = from.__frot_kind(fid), id;
    if (kind === 'element') {
      id = g.__frot_create_element(from.__frot_tag(fid));
      from.__frot_attrs(fid).forEach(function (a) {
        g.__frot_set_attr(id, a[0], a[1]);
      });
    } else if (kind === 'document-fragment') id = g.__frot_create_fragment();
    else if (kind === 'comment') id = g.__frot_create_comment(from.__frot_text(fid));
    else id = g.__frot_create_text(from.__frot_text(fid));
    pairs.push([fid, id]);
    if (deep)
      from.__frot_children(fid).forEach(function (c) {
        g.__frot_insert_child(id, copy(from, c, true, pairs), null);
      });
    return id;
  }
  // Adopt `node` into this document if it belongs to another window's —
  // `keep` copies it instead (importNode), deep or not — and return what this
  // document holds.
  function own(node, keep, deep) {
    var from = home(node);
    if (!from) return node;
    var pairs = [], id = copy(from, from.__frot_slots(node).id, !keep || deep, pairs);
    if (keep) return g.__frot_wrap(id);
    from.__frot_detach(pairs[0][0]);
    pairs.forEach(function (p) {
      var w = from.__frot_wrap(p[0]);
      g.__frot_rebind(w, p[1]);
      g.__frot_registry.adopt(w, from.__frot_registry.release(w));
    });
    return node;
  }
  Object.defineProperty(g, '__frot_own', { value: g.__frot_brand(own, '__frot_own'), configurable: true });
})(globalThis);
