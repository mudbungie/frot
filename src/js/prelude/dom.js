// DOM facade — the node interface chain, the one wrapper per arena node and
// the collections, over the syscalls (the document's members are in doc.js). A wrapper is a thin object
// around an integer NodeId, which lives in brand.js's instance-state WeakMap
// rather than on the wrapper (bl-3bdc: a real element owns no properties at
// all); the arena is the one source of truth (js.md §2), so every read/write
// is a fresh syscall and wrappers hold no cached arena state. The members of
// Node are in node.js; Element's are in elem.js, elem2.js and the modules that
// implement them.
(function (g) {
  'use strict';
  // A node wrapper's arena id, like every other instance's backing state, lives
  // in brand.js's one WeakMap: a real element owns no properties at all, and
  // `_id` was one every page could read off `Object.getOwnPropertyNames`
  // (bl-3bdc, identity.md §3.16).
  var slots = g.__frot_slots;

  var iface = g.__frot_iface;

  // --- The interface chain (bl-78cb, identity.md §3.21) ----------------------
  // One prototype per node kind, linked as on the binary: EventTarget <- Node <-
  // Element <- HTMLElement <- HTML<Tag>Element, Node <- CharacterData <- Text |
  // Comment, Node <- DocumentType | DocumentFragment | Document <- HTMLDocument.
  // Each is a real interface
  // (`new` throws "Illegal constructor."; Text, Comment and DocumentFragment
  // construct, as Firefox's do), so
  // `instanceof` is the engine's own prototype walk and a text node has no
  // element member to find. Members are added by the modules that implement
  // them, each on the interface the binary puts it on. events.js links Node
  // under EventTarget, which it defines.
  function sub(name, parent, body) {
    return g.__frot_extends(iface(name, null, body), parent);
  }
  var Node = iface('Node');
  var Element = sub('Element', Node);
  // The HTML element interfaces are [HTMLConstructor]s: `new` (or `super()`
  // from a class) constructs only a defined custom element — ce.js answers.
  function html(name) {
    return function (inst, args, target) {
      return g.__frot_html_construct(target, name);
    };
  }
  var HTMLElement = sub('HTMLElement', Element, html('HTMLElement'));
  var CharacterData = sub('CharacterData', Node);
  // `new Text(data)`, `new Comment(data)` and `new DocumentFragment()` make the
  // detached arena node `document.create*` does; the object IS its wrapper.
  function made(name, parent, create) {
    return sub(name, parent, function (inst, args) {
      adopt(inst, g[create](args.length ? String(args[0]) : ''));
    }).prototype;
  }
  var KIND = {
    text: made('Text', CharacterData, '__frot_create_text'),
    comment: made('Comment', CharacterData, '__frot_create_comment'),
    doctype: sub('DocumentType', Node).prototype,
    'document-fragment': made('DocumentFragment', Node, '__frot_create_fragment'),
    document: sub('HTMLDocument', sub('Document', Node)).prototype,
  };

  // Which interface a tag's element implements — read off Firefox 153.0esr by
  // `Object.getPrototypeOf(document.createElement(tag))` for each tag (never
  // from the specification). The '' row is the tags that are plain HTMLElement;
  // a tag in no row is HTMLUnknownElement, and one with a hyphen (a custom
  // element name) is plain HTMLElement. The arena keeps local names only, so an
  // element's namespace is unknown: `svg` alone is SVGElement (identity.md §11).
  var TAGS = {
    '': 'section article nav header footer main b i em strong code noscript summary dt dd small sub sup u s ' +
      'abbr cite dfn kbd mark ruby rt rp samp var wbr bdi bdo address aside figure figcaption hgroup search ' +
      'strike tt big acronym nobr noembed noframes plaintext image rb rtc basefont center',
    Html: 'html', Head: 'head', Body: 'body', Div: 'div', Span: 'span', Paragraph: 'p', Anchor: 'a',
    Image: 'img', Canvas: 'canvas', Input: 'input', Script: 'script', Form: 'form', TextArea: 'textarea',
    Select: 'select', IFrame: 'iframe', Link: 'link', Style: 'style', Button: 'button', Option: 'option',
    Label: 'label', Meta: 'meta', Title: 'title', Template: 'template', Area: 'area', UList: 'ul',
    OList: 'ol', LI: 'li', Table: 'table', TableRow: 'tr', TableCell: 'td th',
    TableSection: 'thead tbody tfoot', TableCaption: 'caption', TableCol: 'col colgroup',
    Heading: 'h1 h2 h3 h4 h5 h6', BR: 'br', HR: 'hr', Pre: 'pre xmp listing', Media: '', Video: 'video',
    Audio: 'audio', Source: 'source', Picture: 'picture', Base: 'base', Details: 'details',
    Dialog: 'dialog', FieldSet: 'fieldset', Legend: 'legend', Object: 'object', Embed: 'embed',
    Map: 'map', DList: 'dl', Quote: 'blockquote q', Mod: 'ins del', Time: 'time', Data: 'data',
    Output: 'output', Progress: 'progress', Meter: 'meter', OptGroup: 'optgroup', DataList: 'datalist',
    Slot: 'slot', Font: 'font', Marquee: 'marquee', FrameSet: 'frameset', Frame: 'frame', Menu: 'menu',
    Param: 'param', Track: 'track', Directory: 'dir', Unknown: '',
  };
  var byTag = Object.create(null);
  Object.keys(TAGS).forEach(function (n) {
    var parent = n === 'Video' || n === 'Audio' ? g.HTMLMediaElement : HTMLElement;
    var C = n ? sub('HTML' + n + 'Element', parent, html('HTML' + n + 'Element')) : HTMLElement;
    TAGS[n].split(' ').forEach(function (tag) {
      byTag[tag] = C.prototype;
    });
  });
  byTag.svg = sub('SVGElement', Element).prototype;
  delete byTag[''];

  // --- One wrapper per node (bl-78cb) ----------------------------------------
  // The arena is append-only and a node's kind never changes, so the wrapper of
  // an id is made once and found again: `el === el`, a property a page sets on
  // a node is still there at the next read, and the prototype is chosen once
  // rather than at every access. A wrapper still holds no arena state — only
  // its id, in brand.js's store.
  var wrappers = [];
  function adopt(w, id) {
    slots(w).id = id;
    wrappers[id] = w;
    return w;
  }
  function wrap(id) {
    if (id === null || id === undefined) return null;
    var w = wrappers[id];
    if (w !== undefined) return w;
    var tag = g.__frot_tag(id);
    var proto =
      tag == null
        ? KIND[g.__frot_kind(id)]
        : byTag[tag] || (tag.indexOf('-') < 0 ? g.HTMLUnknownElement : HTMLElement).prototype;
    return adopt(Object.create(proto), id);
  }
  // Define one member on each named HTML<Name>Element prototype: the home of
  // every tag-specific member (`value`, `href`, `getContext`...), which the
  // binary puts on those interfaces and on no shared one.
  function tagdef(names, key, desc) {
    desc.enumerable = true;
    desc.configurable = true;
    names.split(' ').forEach(function (n) {
      Object.defineProperty(g['HTML' + n + 'Element'].prototype, key, desc);
    });
  }
  [['__frot_wrap', wrap], ['__frot_tagdef', tagdef]].forEach(function (p) {
    Object.defineProperty(g, p[0], { value: g.__frot_brand(p[1], p[0]), configurable: true, writable: true });
  });

  // --- DOM collections: one maker, two spec-named interfaces (bl-e5c3) -------
  // The invariant that dissolves the collection zoo: every collection a DOM
  // query returns is an instance of the interface the web spec names for it —
  // childNodes / querySelectorAll / getElementsByName yield a NodeList,
  // children / getElementsByTagName / getElementsByClassName an HTMLCollection
  // — and both are built by this one maker: a snapshot of wrapped nodes as own
  // indexed props over the interface prototype. Like every wrapper, a
  // collection holds no arena state beyond the nodes it was asked to hold.
  // WebIDL shapes the prototypes: `new NodeList()` throws Illegal constructor
  // (via __frot_iface), `length` is a prototype accessor, and the iteration
  // methods ARE the Array.prototype ones — exactly as Gecko exposes them.
  var A = Array.prototype;
  var hasOwn = Object.prototype.hasOwnProperty;
  function item(i) {
    var n = this[i >>> 0];
    return n === undefined ? null : n;
  }
  function collectionLength() {
    var n = 0;
    while (hasOwn.call(this, n)) n++;
    return n;
  }
  // Methods first, `length` after: that is the own-property order Gecko's
  // `NodeList.prototype`/`HTMLCollection.prototype` enumerate in (measured,
  // identity.md §3.15 — operations, then attributes, then `constructor`).
  function defineCollection(name, methods) {
    var Ctor = g.__frot_iface(name);
    Object.keys(methods).forEach(function (k) {
      Object.defineProperty(Ctor.prototype, k, {
        value: g.__frot_brand(methods[k], k),
        writable: true,
        enumerable: true,
        configurable: true,
      });
    });
    g.__frot_ifaceattrs(Ctor.prototype, { length: collectionLength });
    Object.defineProperty(Ctor.prototype, Symbol.iterator, {
      value: A.values,
      writable: true,
      configurable: true,
    });
    return Ctor;
  }
  var NodeList = defineCollection('NodeList', {
    item: item,
    keys: A.keys,
    values: A.values,
    entries: A.entries,
    forEach: A.forEach,
  });
  var HTMLCollection = defineCollection('HTMLCollection', {
    item: item,
    namedItem: function namedItem(name) {
      var s = String(name);
      for (var i = 0; hasOwn.call(this, i); i++) {
        var n = this[i];
        if (n.getAttribute('id') === s || n.getAttribute('name') === s) return n;
      }
      return null;
    },
  });
  function collect(Ctor, ids) {
    var c = Object.create(Ctor.prototype);
    for (var i = 0; i < ids.length; i++) {
      Object.defineProperty(c, i, { value: wrap(ids[i]), enumerable: true, configurable: true });
    }
    return c;
  }
  function wrapAll(ids) {
    return collect(NodeList, ids);
  }
  // The HTMLCollection maker, shared with elem2.js's getElementsBy* (the same
  // non-enumerable-global seam brand.js uses for __frot_brand/__frot_iface).
  Object.defineProperty(g, '__frot_elems', {
    value: g.__frot_brand(function (ids) {
      return collect(HTMLCollection, ids);
    }, '__frot_elems'),
    configurable: true,
    writable: true,
  });
  // The NodeList maker, shared with observer.js's MutationRecord
  // addedNodes/removedNodes (bl-07ab) — the same seam pattern as __frot_elems.
  Object.defineProperty(g, '__frot_nodelist', {
    value: g.__frot_brand(wrapAll, '__frot_nodelist'),
    configurable: true,
    writable: true,
  });

  // Detach every child of `node`. A module-local function published on the same
  // non-enumerable seam as `__frot_elems`, NOT a `_clear` method on
  // `Node.prototype`: a page reads that prototype's own-property list, and
  // Gecko's carries no such member (bl-3bdc).
  function clear(node) {
    var kids = g.__frot_children(slots(node).id);
    for (var i = 0; i < kids.length; i++) g.__frot_detach(kids[i]);
  }
  Object.defineProperty(g, '__frot_clear', {
    value: g.__frot_brand(clear, '__frot_clear'),
    configurable: true,
    writable: true,
  });

  // The document is arena node `__frot_document` (bl-196a), so `document` is
  // that node's wrapper like any other (bl-9f79): the tree operations come
  // from Node.prototype and answer for it by id.
  g.document = wrap(g.__frot_document);
})(globalThis);
