// Deterministic pixel machinery for the WebGL fingerprint masquerade (bl-f624,
// identity.md §11). frot runs no GPU; like canvas.js's 2D bitmap, a WebGL
// `readPixels`/`toDataURL` result is a DETERMINISTIC digest expansion — the same
// draw sequence yields identical pixels every invocation, a changed one diverges,
// never random per call (the tell privacy tools show). Shared by both WebGL
// contexts, so the two shims fold and read through one primitive. Runs after
// brand.js; exposes `__frot_webgl_mix` (FNV-1a digest) and `__frot_webgl_fill`
// (the xorshift bitmap expansion). Pixel realism against a real llvmpipe render is
// the declared residual — determinism and coherence are what a fingerprint reads.
(function (g) {
  'use strict';

  // FNV-1a over a string's UTF-16 code units (folded two bytes each) — the one
  // mixing primitive: context seed and per-call draw digest both fold through it.
  function mix(h, s) {
    h = h >>> 0;
    for (var i = 0; i < s.length; i++) {
      var c = s.charCodeAt(i);
      h = Math.imul(h ^ (c & 0xff), 0x01000193) >>> 0;
      h = Math.imul(h ^ ((c >>> 8) & 0xff), 0x01000193) >>> 0;
    }
    return h >>> 0;
  }

  // Fill `out` (a typed-array view of w*h*4 RGBA bytes) deterministically from
  // (seed, digest, dimensions). An un-drawn surface stays untouched — all-zero,
  // exactly like a real freshly-allocated WebGL buffer read before any draw. A
  // drawn one expands the digest through an xorshift PRNG into opaque noise:
  // stable per (seed, digest, size), content-varying. Out-of-range writes on a
  // short view are silently ignored (typed-array semantics), so a caller's
  // `readPixels` buffer of any length is safe.
  function fill(out, seed, digest, w, h, drawn) {
    if (!drawn) return out;
    var n = w * h;
    var s = (seed ^ digest ^ Math.imul(w, 0x9e3779b1) ^ Math.imul(h, 0x85ebca77)) >>> 0;
    if (s === 0) s = 0x9e3779b1;
    for (var p = 0; p < n; p++) {
      s ^= s << 13;
      s >>>= 0;
      s ^= s >>> 17;
      s ^= s << 5;
      s >>>= 0;
      out[p * 4] = s & 0xff;
      out[p * 4 + 1] = (s >>> 8) & 0xff;
      out[p * 4 + 2] = (s >>> 16) & 0xff;
      out[p * 4 + 3] = 255;
    }
    return out;
  }

  [
    ['__frot_webgl_mix', mix],
    ['__frot_webgl_fill', fill],
  ].forEach(function (pair) {
    g.__frot_brand(pair[1], pair[0]);
    Object.defineProperty(g, pair[0], { value: pair[1], configurable: true, writable: true });
  });
})(globalThis);
