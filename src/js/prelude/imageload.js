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
  var raw = { set: g.__frot_set_attr, remove: g.__frot_remove_attr, fragment: g.__frot_fragment };
  var states = Object.create(null); // <img> node id -> its current request
  var queue = []; // [id, generation, background, sent] image tasks, in order
  var fetch = g.__frot_image_fetch; // bound only under --images
  var lazy = fetch && new g.IntersectionObserver(release, { rootMargin: '600px' });

  function isSrc(id, name) {
    return String(name).toLowerCase() === 'src' && g.__frot_tag(id) === 'img';
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
    var src = g.__frot_attr(id, 'src');
    st.url = src ? g.__frot_wrap(id).src : '';
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
    g.__frot_query(root, 'img[src]').forEach(request);
  }

  function seam(name, fn) {
    Object.defineProperty(g, name, { value: brand(fn, name), configurable: true, writable: true });
  }
  seam('__frot_set_attr', function (id, name, value) {
    raw.set(id, name, value);
    if (isSrc(id, name)) request(id);
  });
  seam('__frot_remove_attr', function (id, name) {
    var had = g.__frot_attr(id, String(name).toLowerCase()) != null;
    raw.remove(id, name);
    if (had && isSrc(id, name)) request(id);
  });
  seam('__frot_fragment', function (html, context) {
    var f = raw.fragment(html, context);
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
  // Image tasks are due now, so they run ahead of any timer.
  var nextTimer = g.__frot_next_timer;
  seam('__frot_next_timer', function () {
    return queue.length ? g.__frot_next_image() : nextTimer();
  });

  // What image.js publishes on HTMLImageElement.prototype, by node id.
  var hook = {
    complete: function (id) {
      var st = states[id];
      return !st || !st.pending;
    },
    natural: function (id, i) {
      var st = states[id];
      return st && st.size ? st.size[i] : 0;
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
