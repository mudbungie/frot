// Deterministic sample machinery for the Web Audio fingerprint masquerade
// (bl-8733, identity.md §11). frot runs no audio DSP; like canvas.js's 2D bitmap
// and webglpix.js's readPixels, an OfflineAudioContext's rendered buffer is a
// DETERMINISTIC digest expansion — the same graph yields identical float samples
// every invocation, a changed graph diverges, never random per call (the tell
// privacy tools show). Shared by the contexts (audio.js) and the node zoo /
// AudioBuffer (audionode.js), so all three fold and read through one primitive.
// Runs after brand.js; exposes `__frot_audio_mix` (FNV-1a digest), the one
// mixing primitive every graph mutation folds through; `__frot_audio_render`
// (the xorshift float expansion, samples in [-1, 1]); and `__frot_audio_class`
// (a branded, CONSTRUCTABLE Firefox-shaped interface helper, shared so the two
// node/context modules shape their classes identically). Waveform realism
// against a real Gecko render is the declared residual — determinism and a
// plausible range are what a fingerprint reads. No syscall (determinism forbids
// entropy).
(function (g) {
  'use strict';
  var brand = g.__frot_brand;

  // FNV-1a over a string's UTF-16 code units (folded two bytes each) — the one
  // mixing primitive: the audio_seed, each context's graph digest, and every
  // param/connect/start mutation all fold through it, exactly as canvas/webgl do.
  function mix(h, s) {
    h = h >>> 0;
    for (var i = 0; i < s.length; i++) {
      var c = s.charCodeAt(i);
      h = Math.imul(h ^ (c & 0xff), 0x01000193) >>> 0;
      h = Math.imul(h ^ ((c >>> 8) & 0xff), 0x01000193) >>> 0;
    }
    return h >>> 0;
  }

  // A rendered channel's samples: `length` floats deterministically expanded from
  // (seed, the frozen graph digest, the channel index) through an xorshift PRNG,
  // each mapped into [-1, 1] — plausible audio, stable per (seed, digest, length,
  // channel), graph-varying, never random. The same digest ALWAYS yields the same
  // samples, so the fingerprint hash a page computes is identical across runs.
  function render(seed, digest, length, channel) {
    var out = new Float32Array(length);
    var s = (seed ^ digest ^ Math.imul(channel + 1, 0x9e3779b1)) >>> 0;
    if (s === 0) s = 0x9e3779b1;
    for (var i = 0; i < length; i++) {
      s ^= s << 13;
      s >>>= 0;
      s ^= s >>> 17;
      s ^= s << 5;
      s >>>= 0;
      out[i] = (s >>> 0) / 4294967295 * 2 - 1;
    }
    return out;
  }

  // A branded, CONSTRUCTABLE Firefox-shaped interface: `new Name(...)` runs
  // `construct(inst, arguments)` to shape the instance; @@toStringTag gives
  // `[object Name]`; the constructor is a NON-enumerable global (as browsers
  // expose their interfaces) and reads as native code. Unlike __frot_iface (whose
  // ctor throws "Illegal constructor"), audio's contexts, nodes, and AudioBuffer
  // ARE constructable in Firefox, so they build through this shared helper.
  function klass(name, construct) {
    var holder = {};
    holder[name] = function () {
      construct(this, arguments);
    };
    var C = brand(holder[name], name);
    Object.defineProperty(C.prototype, Symbol.toStringTag, {
      value: name,
      configurable: true,
    });
    Object.defineProperty(g, name, { value: C, configurable: true, writable: true });
    return C;
  }

  [
    ['__frot_audio_mix', mix],
    ['__frot_audio_render', render],
    ['__frot_audio_class', klass],
  ].forEach(function (pair) {
    brand(pair[1], pair[0]);
    Object.defineProperty(g, pair[0], { value: pair[1], configurable: true, writable: true });
  });
})(globalThis);
