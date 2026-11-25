// The in-page probes of scripts/fidelity-record: each function is shipped to
// Chrome as source and returns Chrome's own answer for its channels
// (docs/design/fidelity.md §2.2). They run in the page, not in Node.
'use strict';
/* global document, getComputedStyle, innerWidth, innerHeight, ShadowRoot */

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
//
// Every channel walks the flat tree (js.md §15): a host's shadow root in place
// of its children, a slot's assigned nodes in place of its own. On a page with
// no shadow root that is the document's own pre-order, the order
// `querySelectorAll('*')` answers in. A closed shadow root is out of a page
// script's reach, so the corpus uses open ones.
function composed(root) {
  const kids = (n) => {
    if (n.shadowRoot) return [...n.shadowRoot.childNodes];
    if (n.localName === 'slot' && n.getRootNode() instanceof ShadowRoot) {
      const assigned = n.assignedNodes();
      if (assigned.length) return assigned;
    }
    return [...n.childNodes];
  };
  const all = [];
  const walk = (n) => {
    for (const c of kids(n)) {
      if (c.nodeType === 1) all.push(c);
      walk(c);
    }
  };
  walk(root);
  return { kids, all };
}

function flat() {
  const { all } = composed(document);
  return {
    links: all.filter((e) => e.matches(':any-link')).map((a) => ({ tag: a.localName,
      href: typeof a.href === 'string' ? a.href : new URL(a.href.baseVal, document.baseURI).href,
      text: a.textContent })),
    forms: all.filter((e) => e.localName === 'form').map((f) => ({
      action: f.action,
      method: f.method,
      elements: [...f.elements].map((e) => ({ tag: e.localName, type: e.type,
        typeAttr: e.getAttribute('type'), name: e.getAttribute('name') })) })),
    boxes: all.map((e) => ({ tag: e.localName,
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
//
// `innerText` does not see into a shadow tree either (js.md §15.1): on a page
// with one, the body's flat tree is rebuilt as plain elements, each carrying
// the computed `display`, `visibility`, `text-transform` and `white-space` of
// the node it stands for (a custom element or a slot as a span, attributes
// but `id`, `class` and `style` kept), and `innerText` is asked of that.
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
  const { kids, all } = composed(document.body);
  for (const e of all) {
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
  if (!all.some((e) => e.shadowRoot)) return document.body.innerText;
  const plain = (n) => {
    if (n.nodeType === 3) return document.createTextNode(n.data);
    if (n.nodeType !== 1) return null;
    const custom = n.localName.includes('-') || n.localName === 'slot';
    const m = document.createElement(custom ? 'span' : n.localName);
    for (const a of n.attributes) {
      if (!['id', 'class', 'style', 'is'].includes(a.name)) m.setAttribute(a.name, a.value);
    }
    const style = getComputedStyle(n);
    Object.assign(m.style, { display: style.display, visibility: style.visibility,
      textTransform: style.textTransform, whiteSpace: style.whiteSpace });
    for (const c of kids(n)) {
      const k = plain(c);
      if (k) m.appendChild(k);
    }
    return m;
  };
  const copy = plain(document.body);
  document.body.replaceWith(copy);
  return copy.innerText;
}

module.exports = { flat: `${composed};(${flat})()`, text: `${composed};(${text})()` };
