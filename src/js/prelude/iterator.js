// The two ES iterator helpers the engine leaks through (bl-5249, found live on
// astro.build: `qsa('.integration-tab').values().find(...)`). quickjs-ng's C
// `Iterator.prototype.find` and `.filter` never release a value their predicate
// REJECTS - each skipped value keeps a reference C code alone can drop, so
// engine teardown finds a non-empty GC list and aborts the PROCESS
// (`JS_FreeRuntime: Assertion list_empty(&rt->gc_obj_list) failed`, SIGABRT, no
// envelope). Upstream fixed `find` in quickjs-ng 0.16.0, but rquickjs still
// ships 0.15.1 and a git pin does not survive publishing, so the two are
// composed here out of the sibling helpers that are free of these leaks:
// `find` is a `some` that keeps the value it stopped on, `filter` a `flatMap`
// yielding one value or none. Laziness, `next` caching, iterator closing,
// suspended-start `return`, and the `Iterator Helper` shape all stay the
// engine's. DELETE THIS FILE when rquickjs ships a quickjs-ng >= 0.16.0 with
// the `filter` free too. Runs after brand.js, so both read as native code.
(function (g) {
  'use strict';

  var proto = g.Iterator.prototype;
  var apply = Reflect.apply;
  // Captured BEFORE the replacement, and invoked through `Reflect.apply`: a
  // page that later overrides `some`/`flatMap` (its own or the prototype's)
  // redirects its own calls, never frot's composition.
  var some = proto.some;
  var flatMap = proto.flatMap;
  // A non-callable predicate is the engine's own case: it validates, closes the
  // iterator (ES2026 27.1.3.3.4-.5), and throws without ever reading a value —
  // so it cannot leak, and delegating keeps that behaviour exactly.
  var find = proto.find;
  var filter = proto.filter;

  // Concise methods: like the built-ins they replace, they are not constructors.
  var replacements = {
    find(predicate) {
      if (typeof predicate !== 'function') return apply(find, this, [predicate]);
      var hit = false;
      var found;
      apply(some, this, [
        function (value, index) {
          hit = !!predicate(value, index);
          found = hit ? value : found;
          return hit;
        },
      ]);
      return hit ? found : undefined;
    },
    filter(predicate) {
      if (typeof predicate !== 'function') return apply(filter, this, [predicate]);
      // One value or none is exactly "keep" or "skip".
      return apply(flatMap, this, [
        function (value, index) {
          return predicate(value, index) ? [value] : [];
        },
      ]);
    },
  };

  Object.keys(replacements).forEach(function (name) {
    Object.defineProperty(proto, name, {
      value: g.__frot_brand(replacements[name], name),
      writable: true,
      configurable: true,
    });
  });
})(globalThis);
