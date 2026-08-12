// Web Audio contexts for the fingerprint masquerade (bl-8733, identity.md
// §10/§11, js.md §7). Firefox exposes AudioContext/OfflineAudioContext; a null
// surface is a louder tell than a costume (§10), so frot returns branded,
// Firefox-shaped contexts. OfflineAudioContext.startRendering() resolves an
// AudioBuffer whose float samples are a DETERMINISTIC function of (the FIXED
// `audioSeed` profile const + the graph digest each node/param mutation folds
// in) — so a fingerprinter's hash is stable across invocations and varies with
// the graph, never random. NO webkit-prefixed alias (Firefox has none). Runs
// after audiobuf.js (render/mix/class) and audionode.js (the node/buffer
// factories). currentTime is a frozen 0 for a realtime context and state stays
// 'suspended' (no audio hardware clock) — declared identity.md §11 residuals, coherent with
// the deterministic persona. No syscall — determinism forbids entropy.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var iface = g.__frot_iface;
  var klass = g.__frot_audio_class;
  var mix = g.__frot_audio_mix;
  var makeNode = g.__frot_audio_node;
  var makeBuffer = g.__frot_audio_buffer;
  var attrs = g.__frot_ifaceattrs;
  // Every context/event value lives in brand.js's one instance-state WeakMap, so
  // a walked context owns nothing — Gecko's shape (bl-3bdc, identity.md §3.16).
  var slots = g.__frot_slots;
  var A = JSON.parse(g.__frot_env_profile()).audio;
  var SEED = A.audioSeed >>> 0;

  // --- BaseAudioContext: the shared surface (Firefox: abstract / Illegal) ------
  var Base = iface('BaseAudioContext', ['sampleRate', 'currentTime', 'state',
    'destination']);
  var CREATE = {
    createOscillator: 'OscillatorNode', createDynamicsCompressor: 'DynamicsCompressorNode',
    createGain: 'GainNode', createBiquadFilter: 'BiquadFilterNode',
    createAnalyser: 'AnalyserNode', createBufferSource: 'AudioBufferSourceNode',
  };
  Object.keys(CREATE).forEach(function (name) {
    Base.prototype[name] = brand(function () { return makeNode(CREATE[name], this); }, name);
  });
  Base.prototype.createBuffer = brand(function (nc, len, sr) {
    return makeBuffer(nc >>> 0, len >>> 0, +sr, false, 0, 0);
  }, 'createBuffer');

  function initContext(inst, kind, sampleRate, maxChannels) {
    var st = slots(inst);
    st.d = mix(SEED, kind);
    st.sampleRate = sampleRate;
    st.currentTime = 0;
    st.state = 'suspended';
    st.maxChannels = maxChannels;
    st.destination = makeNode('AudioDestinationNode', inst);
  }

  // --- AudioContext (realtime, constructable) --------------------------------
  var AudioContext = klass('AudioContext', function (inst) {
    initContext(inst, 'AudioContext', A.sampleRate, A.maxChannelCount);
  });
  Object.setPrototypeOf(AudioContext.prototype, Base.prototype);
  // Fixed profile facts, so the getter IS the value — no slot to stash them in
  // and, unlike the `_baseLatency` prototype properties this replaced, nothing
  // extra for a page reading `AudioContext.prototype`'s own names to find.
  attrs(AudioContext.prototype, {
    baseLatency: function () { return A.baseLatency; },
    outputLatency: function () { return A.outputLatency; },
  });
  ['resume', 'suspend', 'close'].forEach(function (m) {
    AudioContext.prototype[m] = brand(function () { return Promise.resolve(undefined); }, m);
  });

  // --- OfflineAudioContext (constructable) -----------------------------------
  // `new OfflineAudioContext(numberOfChannels, length, sampleRate)` or the option
  // dictionary — the two shapes Firefox accepts; a call with neither throws.
  function offlineArgs(args) {
    if (args.length === 1 && args[0] && typeof args[0] === 'object') {
      var o = args[0];
      return [o.numberOfChannels === undefined ? 1 : o.numberOfChannels >>> 0,
        o.length >>> 0, +o.sampleRate];
    }
    if (args.length < 3) {
      throw new TypeError('OfflineAudioContext constructor requires 3 arguments');
    }
    return [args[0] >>> 0, args[1] >>> 0, +args[2]];
  }
  var OfflineAudioContext = klass('OfflineAudioContext', function (inst, args) {
    var a = offlineArgs(args);
    initContext(inst, 'OfflineAudioContext', a[2], a[0]);
    var st = slots(inst);
    st.numberOfChannels = a[0];
    st.length = a[1];
  });
  Object.setPrototypeOf(OfflineAudioContext.prototype, Base.prototype);
  attrs(OfflineAudioContext.prototype, ['length']);
  g.__frot_onevent(OfflineAudioContext.prototype, 'oncomplete');
  OfflineAudioContext.prototype.suspend = brand(function () { return Promise.resolve(undefined); }, 'suspend');

  // OfflineAudioCompletionEvent: an Event subtype carrying the rendered buffer,
  // so `ctx.oncomplete = e => e.renderedBuffer` (the fingerprintjs shape) works.
  var CompletionEvent = klass('OfflineAudioCompletionEvent', function (inst, args) {
    var o = args[1] || {};
    var st = slots(inst);
    st.type = String(args[0]);
    st.renderedBuffer = o.renderedBuffer === undefined ? null : o.renderedBuffer;
  });
  Object.setPrototypeOf(CompletionEvent.prototype, g.Event.prototype);
  attrs(CompletionEvent.prototype, ['type', 'renderedBuffer']);

  // startRendering(): the rendered buffer's digest is FROZEN to the graph built so
  // far, so repeat calls and independent invocations agree. Resolves the promise
  // AND fires oncomplete on the microtask queue (both Firefox delivery paths),
  // which the run driver drains before it reports the run settled.
  OfflineAudioContext.prototype.startRendering = brand(function () {
    var st = slots(this);
    var buf = makeBuffer(st.numberOfChannels, st.length, st.sampleRate, true, SEED, st.d);
    st.currentTime = st.sampleRate ? st.length / st.sampleRate : 0;
    st.state = 'closed';
    var self = this;
    Promise.resolve().then(function () {
      var cb = self.oncomplete;
      if (typeof cb === 'function') cb(new CompletionEvent('complete', { renderedBuffer: buf }));
    });
    return Promise.resolve(buf);
  }, 'startRendering');
})(globalThis);
