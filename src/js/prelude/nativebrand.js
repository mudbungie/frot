// Final native-code sweep (identity.md §8, bl-3972 / bl-3926). frot's web APIs
// are implemented in the JS prelude over `__frot_*` syscalls, so an un-branded
// `fetch.toString()` / `querySelector.toString()` would disclose prelude source
// and the syscall name `frot`. This runs LAST and routes the whole existing
// surface through the ONE registry brand.js owns — no second toString wrapper,
// no per-call-site patch. Page scripts have not run yet, so every function
// reachable here is either a genuine engine built-in or a frot web API; both
// legitimately read as `[native code]`. The six later capability balls
// (canvas/WebGL/audio/indexedDB/Worker/permissions) brand their own additions
// via __frot_brand as they define them.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;

  // Brand every function-valued and accessor property directly on `obj`.
  function sweep(obj) {
    Object.getOwnPropertyNames(obj).forEach(function (k) {
      var d = Object.getOwnPropertyDescriptor(obj, k);
      if (!d) return;
      if (typeof d.value === 'function') brand(d.value, d.value.name || k);
      if (typeof d.get === 'function') brand(d.get, 'get ' + k);
      if (typeof d.set === 'function') brand(d.set, 'set ' + k);
    });
  }

  // Walk an object's own props then its prototype chain up to (not including)
  // Object.prototype — reaching the Node/Element/Document methods a page probes.
  function sweepChain(obj) {
    while (obj && obj !== Object.prototype) {
      sweep(obj);
      obj = Object.getPrototypeOf(obj);
    }
  }

  // `constructor` last on every interface prototype (identity.md §3.15). Here
  // because here is where every prototype is finally complete: the registry
  // records each interface as it is built, and this is the one place that runs
  // after all of them. `EventTarget` and `Worker` are passed explicitly — they
  // are hand-rolled constructors rather than `__frot_iface` products, so the
  // registry never saw them.
  //
  // First, the DOM interfaces whose members several modules define (bl-78cb):
  // each is put in the order Firefox 153.0esr's prototype lists the members
  // frot implements (identity.md §3.21), and made enumerable as WebIDL members
  // are. One pass here instead of a definition order threaded through a dozen
  // files; `tests/fixtures/js/dom-interfaces.json` holds the measured lists a
  // test checks every prototype against.
  // GlobalEventHandlers (handlers.js), whose `onerror` the binary lists last:
  // after `dataset` on an element, after the other handlers on a document.
  var ON = g.__frot_handler_names.replace(' onerror', '');
  var ORDER = {
    Element: 'getAttributeNames getAttribute getAttributeNS toggleAttribute setAttribute setAttributeNS ' +
      'removeAttribute removeAttributeNS hasAttribute hasAttributes closest matches getElementsByTagName ' +
      'getElementsByClassName getBoundingClientRect attachShadow getHTML remove prepend append querySelector ' +
      'querySelectorAll localName tagName id className classList attributes clientWidth clientHeight ' +
      'innerHTML outerHTML shadowRoot assignedSlot slot previousElementSibling nextElementSibling ' +
      'children firstElementChild lastElementChild childElementCount',
    HTMLElement: 'focus blur title style offsetTop offsetLeft offsetWidth offsetHeight ' + ON + ' dataset onerror',
    SVGElement: 'focus blur style ' + ON + ' dataset onerror',
    CharacterData: 'remove data length previousElementSibling nextElementSibling',
    DocumentFragment: 'getElementById prepend append querySelector querySelectorAll children ' +
      'firstElementChild lastElementChild childElementCount',
    Document: 'getElementsByTagName getElementsByClassName createElement createElementNS ' +
      'createDocumentFragment createTextNode createComment importNode adoptNode createEvent ' +
      'createTreeWalker getElementsByName ' +
      'hasFocus getElementById prepend append querySelector querySelectorAll implementation URL ' +
      'documentURI compatMode doctype documentElement referrer cookie ' +
      'readyState title body head forms scripts defaultView onreadystatechange currentScript hidden ' +
      'visibilityState activeElement adoptedStyleSheets ' + ON + ' onerror children firstElementChild ' +
      'lastElementChild childElementCount',
    HTMLLinkElement: 'disabled href rel relList type',
    HTMLFieldSetElement: 'disabled form name type elements',
    HTMLIFrameElement: 'src srcdoc name width height contentDocument contentWindow',
    HTMLFormElement: 'action name rel relList elements length',
    HTMLObjectElement: 'data type name useMap form width height contentDocument contentWindow',
    HTMLLIElement: 'value type',
    HTMLParamElement: 'value type',
    HTMLScriptElement: 'src type text',
    HTMLImageElement: 'decode alt src srcset crossOrigin useMap referrerPolicy isMap width height ' +
      'decoding loading fetchPriority naturalWidth naturalHeight complete name align hspace vspace ' +
      'longDesc border sizes currentSrc lowsrc x y',
  };
  // The tag-specific members sit in this relative order on every HTML<Tag>
  // interface but the four named above.
  var TAGGED = 'getContext toDataURL defaultChecked checked disabled form indeterminate name src srcdoc rel ' +
    'relList href ' +
    'type defaultValue defaultSelected selected ' +
    'value width height contentDocument contentWindow';
  Object.getOwnPropertyNames(g).forEach(function (name) {
    if (!ORDER[name] && !/^HTML.+Element$/.test(name)) return;
    var proto = g[name].prototype;
    (ORDER[name] || TAGGED).split(' ').forEach(function (k) {
      var d = Object.getOwnPropertyDescriptor(proto, k);
      if (!d) return;
      delete proto[k];
      d.enumerable = true;
      Object.defineProperty(proto, k, d);
    });
  });
  var built = g.__frot_iface_seal([g.EventTarget, g.Worker, g.SharedWorker, g.Notification,
    g.AbortSignal, g.AbortController, g.Event, g.CustomEvent]);

  sweep(g); // window's own methods + the raw __frot_* syscalls (source hidden)
  // Every node interface, not only the ones one sample element reaches.
  built.forEach(function (C) {
    if (C.prototype instanceof g.EventTarget) sweep(C.prototype);
  });
  [g.document, g.navigator, g.screen, g.crypto, g.location, g.history].forEach(function (o) {
    if (o) sweepChain(o);
  });
})(globalThis);
