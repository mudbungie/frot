// Markup <-> arena (js.md §2/§3, bl-273b): the ONE place nodes become an HTML
// string and an HTML string becomes nodes. `innerHTML` and `outerHTML` are the
// two directions of one fact, so they live together here rather than beside the
// handle plumbing in dom.js — and there is exactly one serializer, shared, so a
// read-modify-write (`el.innerHTML = el.innerHTML`) cannot lose anything one
// getter happened to know about and another did not.
//
// The bug this file exists for: the old getter was `'<' + tag + '>' + inner +
// '</' + tag + '>'`, so `<div id=x class=y>` read back as `<div>` — every
// attribute silently deleted by the round trip, `<br>` grown a `</br>`, and
// `&`/`<` in text handed back as live markup. Serialization follows WHATWG HTML
// §13.3 ("HTML fragment serialization algorithm"), verified against Chrome 139.
//
// Loads after dom.js (it extends the Node prototype dom.js defines) and before
// anything that reads markup.
(function (g) {
  'use strict';
  var proto = g.Node.prototype;

  // §13.3 "escaping a string": `&` and U+00A0 always, plus `<`/`>` (the 2023
  // mXSS amendment — Chrome 139 serializes an attribute value `<` as `&lt;`);
  // `"` only inside an attribute value, where it would end the value.
  function esc(s, attr) {
    s = String(s)
      .replace(/&/g, '&amp;')
      .replace(/\u00a0/g, '&nbsp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
    return attr ? s.replace(/"/g, '&quot;') : s;
  }

  // Void elements serialize with no end tag and no self-closing slash
  // (`<br>`, never `<br></br>` or `<br/>`). Raw-text elements' text children are
  // NOT escaped: a `<` in a <script> or <style> body stays a `<`, because
  // re-parsing that content never sees markup. Both lists are the spec's.
  var VOID = ' area base br col embed hr img input link meta source track wbr ';
  var RAW = ' script style xmp iframe noembed noframes plaintext noscript ';
  function listed(set, tag) {
    return !!tag && set.indexOf(' ' + tag + ' ') >= 0;
  }

  // Every read is a fresh syscall against the arena (js.md §2) — the serializer
  // holds no state and caches nothing, like every other wrapper here.
  function nodes(ids, raw) {
    var out = '';
    for (var i = 0; i < ids.length; i++) out += node(ids[i], raw);
    return out;
  }
  function node(id, raw) {
    var kind = g.__frot_kind(id);
    if (kind === 'comment') return '<!--' + g.__frot_text(id) + '-->';
    // Text — and the doctype, whose text is '', so it needs no case of its own.
    if (kind !== 'element') return raw ? g.__frot_text(id) : esc(g.__frot_text(id), false);
    var tag = g.__frot_tag(id);
    var attrs = g.__frot_attrs(id);
    var out = '<' + tag;
    for (var i = 0; i < attrs.length; i++) {
      out += ' ' + attrs[i][0] + '="' + esc(attrs[i][1], true) + '"';
    }
    out += '>';
    if (listed(VOID, tag)) return out;
    return out + nodes(g.__frot_children(id), listed(RAW, tag)) + '</' + tag + '>';
  }

  Object.defineProperty(proto, 'innerHTML', {
    configurable: true,
    get: function () {
      return nodes(g.__frot_children(this._id), listed(RAW, g.__frot_tag(this._id)));
    },
    set: function (html) {
      this._clear();
      var ids = g.__frot_fragment(String(html));
      for (var i = 0; i < ids.length; i++) g.__frot_insert_child(this._id, ids[i], null);
    },
  });

  // `outerHTML` is writable in a browser: the assignment REPLACES the element
  // (and its subtree) with the parsed fragment, in its place among its siblings.
  // Getter-only made that a silent no-op in a classic script since bl-0679 (and
  // a TypeError in a module), so a widget that swaps itself out simply stayed.
  // Replacement is insert-then-detach through the §2 syscalls: each parsed node
  // goes in *before* this one — a reference sibling, never an index (bl-ae88) —
  // and only then does this one leave, so an empty string is not a special case,
  // it is the general path with nothing to insert.
  Object.defineProperty(proto, 'outerHTML', {
    configurable: true,
    get: function () {
      return node(this._id, false);
    },
    set: function (html) {
      var parent = g.__frot_parent(this._id);
      // A root has no parent to replace it within. Browsers reach the Document
      // node here and swap the documentElement; the arena has no document node
      // (js.md §2), so frot throws the spec's error for a parentless element
      // rather than pretending — loud, not silent.
      if (parent === null || parent === undefined) {
        throw g.__frot_domerror(
          'NoModificationAllowedError',
          'An attempt was made to modify an object where modifications are not allowed'
        );
      }
      var ids = g.__frot_fragment(String(html));
      for (var i = 0; i < ids.length; i++) g.__frot_insert_child(parent, ids[i], this._id);
      g.__frot_detach(this._id);
    },
  });
})(globalThis);
