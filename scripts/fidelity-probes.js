// The in-page probes of scripts/fidelity-record: each function is shipped to
// Chrome as source and returns Chrome's own answer for its channels
// (docs/design/fidelity.md §2.2). They run in the page, not in Node.
'use strict';
/* global document, getComputedStyle, innerWidth, innerHeight */

// `links`, `forms`, `boxes`. An element has a box when it has a client rect
// AND layout reaches it: `checkVisibility()` is false inside
// `content-visibility: hidden` (the body of a closed <details>), where rects
// are still answered but nothing is laid out or painted. Neither half alone
// is the fact — an SVG <defs> child and a <wbr> pass checkVisibility() with no
// rect at all. `visibility: hidden` keeps its box on both halves, as it does
// in frot's `bboxes` (layout.md §7).
//
// `links` is every hyperlink Chrome sees, which is what `:any-link` matches:
// an HTML <a> or <area> with `href`, and an SVG <a> with `href` or
// `xlink:href`. `document.links` is the HTML half alone. An SVG anchor's
// `href` is an SVGAnimatedString holding the attribute as written, so it is
// resolved here the way Chrome resolves it to follow it.
function flat() {
  return {
    links: [...document.querySelectorAll(':any-link')].map((a) => ({ tag: a.localName,
      href: typeof a.href === 'string' ? a.href : new URL(a.href.baseVal, document.baseURI).href,
      text: a.textContent })),
    forms: [...document.forms].map((f) => ({
      action: f.action,
      method: f.method,
      elements: [...f.elements].map((e) => ({ tag: e.localName, type: e.type,
        typeAttr: e.getAttribute('type'), name: e.getAttribute('name') })) })),
    boxes: [...document.querySelectorAll('*')].map((e) => ({ tag: e.localName,
      rects: e.checkVisibility() ? e.getClientRects().length : 0 })),
    viewport: [innerWidth, innerHeight],
  };
}

// `text`. `innerText` is blind to two things Chrome paints: the string content
// of ::before/::after, and a <textarea>'s value. Both are put into the tree as
// real text where they paint — every one read from Chrome first, then all
// inserted, each carrying the three computed properties `innerText` answers to
// — and `innerText` is asked once, after. Only the top-level strings of a
// computed `content` are taken (attr() is already resolved to one there):
// counters, quote marks and images stay unseen, and so does the alternative
// text after a `/`. This rewrites the page, so the recorder runs it last,
// after every other channel has been read.
function text() {
  const strings = (content) => {
    const top = content.replace(/[a-z-]+\((?:"(?:[^"\\]|\\.)*"|[^()"])*\)/gi, '').split(' / ')[0];
    return [...top.matchAll(/"((?:[^"\\]|\\.)*)"/g)]
      .map((m) => m[1].replace(/\\(?:([0-9a-f]{1,6}) ?|(.))/gi,
        (_, hex, ch) => (hex ? String.fromCodePoint(parseInt(hex, 16)) : ch)))
      .join('');
  };
  const todo = [];
  const add = (where, at, value, style, display) => {
    if (value && style.display !== 'none') todo.push({ where, at, value, style, display });
  };
  for (const e of document.body.querySelectorAll('*')) {
    for (const [pseudo, where] of [['::before', 'afterbegin'], ['::after', 'beforeend']]) {
      const style = getComputedStyle(e, pseudo);
      add(where, e, strings(style.content), style, style.display);
    }
    if (e.localName === 'textarea') add('beforebegin', e, e.value, getComputedStyle(e), 'inline');
  }
  for (const { where, at, value, style, display } of todo) {
    const el = document.createElement('fidelity-generated');
    Object.assign(el.style,
      { display, visibility: style.visibility, textTransform: style.textTransform });
    el.textContent = value;
    at.insertAdjacentElement(where, el);
  }
  return document.body.innerText;
}

module.exports = { flat: `(${flat})()`, text: `(${text})()` };
