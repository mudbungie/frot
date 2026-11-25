// What an <img> does with its `src` (bl-91a4; js.md §7 "Images"). Every <img>,
// parsed, created or `new Image()`, follows one rule, the one a Firefox with
// images blocked follows (measured on 153.0esr, `permissions.default.image = 2`):
//
// - setting `src` starts a request: `complete` turns false (an empty `src`
//   leaves it true), and the request before it is abandoned;
// - on a later task the request ends: a `data:` URL holding a whole image frot
//   can size (`__frot_image_size`) LOADS — `naturalWidth`/`naturalHeight` take
//   its size and `load` fires; anything else FAILS — `error` fires, the size
//   stays 0. Without `--images` no request leaves frot for an image, so no
//   beacon is sent;
// - removing `src` abandons the request and fires nothing.
//
// Which URL the request is for is the HTML source selection (bl-0e4a):
// `__frot_image_source` (syscall/image/source.rs) picks among a `<picture>`'s
// `<source>`s, `srcset`/`sizes` and `src` as Firefox does at the persona's
// device pixel ratio of 1, and gives the density the image is shown at, which
// divides its natural size. A change that can change the pick — `src`,
// `srcset` or `sizes` set on the `<img>`, the `<img>` entering or leaving a
// `<picture>`, a `<source>` entering, leaving or changing in one — re-runs it;
// only the `<img>`'s own attributes always start a new request, the rest only
// when the pick moves.
//
// Under `--images` (bl-49f1) the size syscall also knows the network images
// the host fetched: before ending an image task, every queued current request
// not yet sent goes out in one wave (`__frot_image_fetch`, bound only under the
// flag). A `loading=lazy` image is held back, as Firefox holds it, until its
// box comes within 600px of the viewport (an IntersectionObserver); one that
// never does stays pending. `Priority` is the lower one (`background`) for a
// lazy image or one not in the document as its request starts.
//
// A document's `load` waits for its images, as a browser's does: the host runs
// every queued image task before firing it (`__frot_next_image`, js.rs). After
// that they run ahead of the timers. The trigger is the one mutation seam
// observer.js also wraps — the attribute syscalls and the fragment parser — so
// `img.src = x`, `setAttribute`, `cloneNode` and `innerHTML` are all covered
// without a hook per API; the parsed document is scanned once, here.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var brand = g.__frot_brand;
  var raw = {
    set: g.__frot_set_attr,
    remove: g.__frot_remove_attr,
    fragment: g.__frot_fragment,
    insert: g.__frot_insert_child,
    detach: g.__frot_detach,
  };
  var IMG = { src: 1, srcset: 1, sizes: 1 };
  var SOURCE = { srcset: 1, sizes: 1, media: 1, type: 1 };
  var states = Object.create(null); // <img> node id -> its current request
  var queue = []; // [id, generation, background, sent] image tasks, in order
  var fetch = g.__frot_image_fetch; // bound only under --images
  var lazy = fetch && new g.IntersectionObserver(release, { rootMargin: '600px' });

  // What a change to attribute `name` of `id` touches: 'img', 'source', or no.
  function relevant(id, name) {
    var t = g.__frot_tag(id);
    var n = String(name).toLowerCase();
    return (t === 'img' && IMG[n] && t) || (t === 'source' && SOURCE[n] && t);
  }
  // The selected source: [resolved URL, or '' for an empty `src`, or null for
  // nothing to request; the density it is shown at].
  function pick(id) {
    var sel = g.__frot_image_source(id);
    var src = sel ? sel[0] : g.__frot_attr(id, 'src');
    if (!src) return [src, 1];
    try {
      return [new g.URL(src, g.location.href).href, sel ? sel[1] : 1];
    } catch (e) {
      return [src, 1];
    }
  }
  // A change that may move `img`'s pick: a new request only when it does.
  function reselect(id) {
    var st = states[id];
    var now = pick(id);
    var was = st ? [st.url, st.density] : [null, 1];
    if (now[0] !== was[0] && (now[0] || was[0]) || now[1] !== was[1]) request(id);
  }
  // `node` entered or left `parent`: an `<img>` re-picks, and a `<source>`
  // moves the pick of every `<img>` of its `<picture>`.
  function touched(parent, node) {
    var t = g.__frot_tag(node);
    if (t === 'img') reselect(node);
    if (t === 'source' && parent != null && g.__frot_tag(parent) === 'picture') {
      g.__frot_children(parent).forEach(function (c) {
        if (g.__frot_tag(c) === 'img') reselect(c);
      });
    }
  }
  function invalid(message) {
    return g.__frot_domerror('EncodingError', message);
  }
  // Settle every `decode()` waiting on the request `st` held.
  function settle(st, error) {
    var waiting = st.waiters.splice(0);
    for (var i = 0; i < waiting.length; i++) {
      if (error) waiting[i][1](error);
      else waiting[i][0](undefined);
    }
  }

  // "Update the image data": a new request, generation-stamped so a task queued
  // for the one it replaces finds itself stale.
  function request(id) {
    var st = states[id] || (states[id] = { gen: 0, waiters: [] });
    st.gen++;
    settle(st, invalid('Invalid image request.'));
    var sel = pick(id);
    var src = sel[0];
    st.url = src || '';
    st.density = sel[1];
    st.pending = !!src;
    st.size = null;
    var img = g.__frot_wrap(id);
    st.lazy = !!(lazy && src && String(g.__frot_attr(id, 'loading')).toLowerCase() === 'lazy');
    if (st.lazy) lazy.observe(img);
    else if (src != null) queue.push([id, st.gen, !img.isConnected]);
  }
  // The lazy-load observer's callback: a held-back image whose box now meets
  // the margin starts its request.
  function release(entries) {
    entries.forEach(function (e) {
      var st = states[slots(e.target).id];
      if (!e.isIntersecting || !st.lazy) return;
      st.lazy = false;
      lazy.unobserve(e.target);
      queue.push([slots(e.target).id, st.gen, true]);
    });
  }
  // Send every queued current request not sent yet, in one wave.
  function send() {
    var urls = [];
    var bgs = [];
    queue.forEach(function (t) {
      var st = states[t[0]];
      if (t[3] || st.gen !== t[1] || !st.url) return;
      t[3] = true;
      urls.push(st.url);
      bgs.push(t[2]);
    });
    if (urls.length) fetch(urls, bgs);
  }
  function scan(root) {
    g.__frot_query(root, 'img[src], img[srcset], picture > img').forEach(request);
  }

  function seam(name, fn) {
    Object.defineProperty(g, name, { value: brand(fn, name), configurable: true, writable: true });
  }
  function changed(id, name) {
    var r = relevant(id, name);
    if (r === 'img') request(id);
    if (r === 'source') touched(g.__frot_parent(id), id);
  }
  seam('__frot_set_attr', function (id, name, value) {
    raw.set(id, name, value);
    changed(id, name);
  });
  seam('__frot_remove_attr', function (id, name) {
    var had = g.__frot_attr(id, String(name).toLowerCase()) != null;
    raw.remove(id, name);
    if (had) changed(id, name);
  });
  seam('__frot_insert_child', function (parent, child, before) {
    var whole = g.__frot_kind(child) === 'document-fragment';
    var moved = whole ? g.__frot_children(child) : [child];
    var from = whole ? null : g.__frot_parent(child);
    var why = raw.insert(parent, child, before);
    if (!why) {
      moved.forEach(function (m) {
        touched(from, m);
        touched(parent, m);
      });
    }
    return why;
  });
  seam('__frot_detach', function (id) {
    var from = g.__frot_parent(id);
    var r = raw.detach(id);
    touched(from, id);
    return r;
  });
  seam('__frot_fragment', function (html, context, roots) {
    var f = raw.fragment(html, context, !!roots);
    scan(f);
    return f;
  });

  // Host driver: end the oldest queued request — -1 when none is queued, else 0
  // (a handler's throw is reported, and counted, on the dispatch path).
  seam('__frot_next_image', function () {
    if (fetch) send();
    var task = queue.shift();
    if (!task) return -1;
    var id = task[0];
    var st = states[id];
    if (st.gen !== task[1]) return 0;
    st.pending = false;
    st.size = st.url ? g.__frot_image_size(st.url) : null;
    var img = g.__frot_wrap(id);
    var ev = new g.Event(st.size ? 'load' : 'error');
    slots(ev).trusted = true; // the browser raised it, not the page
    g.__frot_dispatch(img, ev);
    settle(st, st.size ? null : invalid('Invalid encoded image data.'));
    return 0;
  });
  // Image tasks are due now, so they run ahead of any timer — but not ahead of
  // DOMContentLoaded: the timers due at the end of parsing (`early`) are not
  // the image's turn.
  var nextTimer = g.__frot_next_timer;
  seam('__frot_next_timer', function (early) {
    return queue.length && !early ? g.__frot_next_image() : nextTimer(early);
  });

  // What image.js publishes on HTMLImageElement.prototype, by node id.
  var hook = {
    complete: function (id) {
      var st = states[id];
      return !st || !st.pending;
    },
    natural: function (id, i) {
      var st = states[id];
      if (!st || !st.size) return 0;
      return st.density ? Math.round(st.size[i] / st.density) : st.size[i];
    },
    currentSrc: function (id) {
      var st = states[id];
      return st ? st.url : '';
    },
    decode: function (id) {
      var st = states[id];
      return new Promise(function (resolve, reject) {
        if (!st || !st.url) reject(invalid('Invalid image request.'));
        else if (st.pending) st.waiters.push([resolve, reject]);
        else if (st.size) resolve(undefined);
        else reject(invalid('Invalid encoded image data.'));
      });
    },
  };
  Object.defineProperty(g, '__frot_image_hook', { value: hook, configurable: true });

  scan(g.__frot_document);
})(globalThis);
