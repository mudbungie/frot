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
// Loads after dom.js (it extends the Element prototype dom.js defines) and before
// anything that reads markup.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var proto = g.Element.prototype;

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

  // Where an element's markup lives: a template's is its contents (js.md §15),
  // never its children — HTML §13.3 serializes the one and ignores the other,
  // and `innerHTML =` fills the contents.
  function inner(id) {
    return g.__frot_tag(id) === 'template' ? g.__frot_content(id) : id;
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
    var is = g.__frot_is_value(id);
    for (var i = 0; i < attrs.length; i++) {
      if (attrs[i][0] === 'is') is = undefined;
      out += ' ' + attrs[i][0] + '="' + esc(attrs[i][1], true) + '"';
    }
    // A customized built-in made by `createElement(tag, {is})` or `new`
    // carries its `is` value in no attribute, and serializes it (js.md §15).
    if (is !== undefined) out += ' is="' + esc(is, true) + '"';
    out += '>';
    if (listed(VOID, tag)) return out;
    return out + nodes(g.__frot_children(inner(id)), listed(RAW, tag)) + '</' + tag + '>';
  }

  Object.defineProperty(proto, 'innerHTML', {
    configurable: true,
    get: function () {
      var id = slots(this).id;
      return nodes(g.__frot_children(inner(id)), listed(RAW, g.__frot_tag(id)));
    },
    set: function (html) {
      var id = inner(slots(this).id);
      g.__frot_clear(g.__frot_wrap(id));
      g.__frot_insert_child(id, g.__frot_fragment(String(html)), null);
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
      return node(slots(this).id, false);
    },
    set: function (html) {
      var parent = g.__frot_parent(slots(this).id);
      // An element in no tree has nothing to be replaced within: a no-op. The
      // document's own child cannot be replaced this way (both measured on
      // Firefox 153.0esr).
      if (parent === null || parent === undefined) return;
      if (parent === g.__frot_document) {
        throw g.__frot_domerror(
          'NoModificationAllowedError',
          'Modifications are not allowed for this document'
        );
      }
      g.__frot_insert_child(parent, g.__frot_fragment(String(html)), slots(this).id);
      g.__frot_detach(slots(this).id);
    },
  });
})(globalThis);
