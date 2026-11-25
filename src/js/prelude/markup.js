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
  // holds no state and caches nothing, like every other wrapper here. `roots`
  // is getHTML's choice of shadow roots to serialize, `null` for none.
  function nodes(ids, raw, roots) {
    var out = '';
    for (var i = 0; i < ids.length; i++) out += node(ids[i], raw, roots);
    return out;
  }
  // A host's shadow root, when `roots` takes it, serializes as a declarative
  // one — the `<template shadowrootmode>` the parser turns back into it.
  var FLAGS = [[5, 'delegatesfocus'], [4, 'serializable'], [3, 'clonable']];
  function shadow(id, roots) {
    var root = roots && g.__frot_shadow(id);
    if (root === null || root === undefined || !roots(root)) return '';
    var info = g.__frot_shadow_info(root);
    var out = '<template shadowrootmode="' + (info[1] ? 'closed' : 'open') + '"';
    FLAGS.forEach(function (f) {
      if (info[f[0]]) out += ' shadowroot' + f[1] + '=""';
    });
    return out + '>' + nodes(g.__frot_children(root), false, roots) + '</template>';
  }
  function node(id, raw, roots) {
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
    out += shadow(id, roots);
    return out + nodes(g.__frot_children(inner(id)), listed(RAW, tag), roots) + '</' + tag + '>';
  }

  // The markup of `id`'s contents — an element's, a template's, a shadow
  // root's. `options` is getHTML's: the shadow roots to serialize are the
  // serializable ones when `serializableShadowRoots` is set, and any listed
  // in `shadowRoots`.
  function markup(id, options) {
    var roots = null;
    if (options) {
      var listed_ = Array.prototype.map.call(options.shadowRoots || [], function (r) {
        return slots(r).id;
      });
      var serializable = !!options.serializableShadowRoots;
      roots = function (root) {
        return (serializable && g.__frot_shadow_info(root)[4]) || listed_.indexOf(root) >= 0;
      };
    }
    return shadow(id, roots) + nodes(g.__frot_children(inner(id)), listed(RAW, g.__frot_tag(id)), roots);
  }
  Object.defineProperty(g, '__frot_markup', {
    value: g.__frot_brand(markup, '__frot_markup'),
    configurable: true,
  });

  Object.defineProperty(proto, 'innerHTML', {
    configurable: true,
    get: function () {
      return markup(slots(this).id);
    },
    set: function (html) {
      // The element itself is the fragment parser's context (bl-538f): a
      // <title>'s markup stays its text.
      var id = inner(slots(this).id);
      g.__frot_clear(g.__frot_wrap(id));
      g.__frot_insert_child(id, g.__frot_fragment(String(html), this.localName), null);
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
      // The parent is the context; a fragment parent parses as a <body>.
      var ctx = g.__frot_kind(parent) === 'element' ? g.__frot_wrap(parent).localName : 'body';
      g.__frot_insert_child(parent, g.__frot_fragment(String(html), ctx), slots(this).id);
      g.__frot_detach(slots(this).id);
    },
  });

  // The fragment parser's context for markup parsed into `id`'s children: the
  // element itself, or a <body> for a fragment, a document or <html> (HTML
  // "insertAdjacentHTML" step 3).
  function contextOf(id) {
    var tag = g.__frot_kind(id) === 'element' ? g.__frot_tag(id) : 'html';
    return tag === 'html' ? 'body' : tag;
  }
  // insertAdjacentHTML (bl-538f), with Firefox 153.0esr's checks and messages:
  // the two outer positions parse in the parent's context and need one that
  // is not the document; the two inner ones parse in the element's own.
  var WHERE = { beforebegin: 1, afterbegin: 1, beforeend: 1, afterend: 1 };
  g.__frot_ifaceops(proto, {
    insertAdjacentHTML: function insertAdjacentHTML(position, text) {
      if (arguments.length < 2) {
        throw new TypeError('Element.insertAdjacentHTML: At least 2 arguments required, but only '
          + arguments.length + ' passed');
      }
      var where = String(position).toLowerCase(), id = slots(this).id;
      if (!WHERE[where]) throw g.__frot_domerror('SyntaxError', 'An invalid or illegal string was specified');
      var outer = where === 'beforebegin' || where === 'afterend';
      var into = outer ? g.__frot_parent(id) : id;
      if (into === null || into === undefined || into === g.__frot_document) {
        throw g.__frot_domerror('NoModificationAllowedError', 'Modifications are not allowed for this document');
      }
      var kids = g.__frot_children(into);
      var ref = { beforebegin: id, afterend: kids[kids.indexOf(id) + 1], afterbegin: kids[0] }[where];
      var frag = g.__frot_fragment(String(text), contextOf(into));
      g.__frot_insert_child(into, frag, ref === undefined ? null : ref);
    },
  });
})(globalThis);
