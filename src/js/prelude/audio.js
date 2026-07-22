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
// 'suspended' (no audio hardware clock) — declared §11 residuals, coherent with
// the deterministic persona. No syscall — determinism forbids entropy.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var iface = g.__frot_iface;
  var klass = g.__frot_audio_class;
  var mix = g.__frot_audio_mix;
  var makeNode = g.__frot_audio_node;
  var makeBuffer = g.__frot_audio_buffer;
  var A = JSON.parse(g.__frot_env_profile()).audio;
  var SEED = A.audioSeed >>> 0;

  function def(o, k, v) {
    Object.defineProperty(o, k, { value: v, configurable: true, writable: true });
  }
  function slot(proto, k) {
    Object.defineProperty(proto, k, {
      get: brand(function () { return this['_' + k]; }, 'get ' + k),
      enumerable: true, configurable: true,
    });
  }

  // --- BaseAudioContext: the shared surface (Firefox: abstract / Illegal) ------
  var Base = iface('BaseAudioContext', {});
  ['sampleRate', 'currentTime', 'state', 'destination'].forEach(function (k) {
    slot(Base.prototype, k);
  });
  // The one graph-digest fold every node/param mutation reaches (non-enumerable,
  // like canvas.js's ctx._d slot — invisible to a page walking the object).
  Object.defineProperty(Base.prototype, '_fold', {
    value: brand(function (s) { this._d = mix(this._d, s); }, '_fold'),
    configurable: true,
  });
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
    def(inst, '_d', mix(SEED, kind));
    def(inst, '_sampleRate', sampleRate);
    def(inst, '_currentTime', 0);
    def(inst, '_state', 'suspended');
    def(inst, '_maxChannels', maxChannels);
    def(inst, '_destination', makeNode('AudioDestinationNode', inst));
  }

  // --- AudioContext (realtime, constructable) --------------------------------
  var AudioContext = klass('AudioContext', function (inst) {
    initContext(inst, 'AudioContext', A.sampleRate, A.maxChannelCount);
  });
  Object.setPrototypeOf(AudioContext.prototype, Base.prototype);
  slot(AudioContext.prototype, 'baseLatency');
  slot(AudioContext.prototype, 'outputLatency');
  Object.defineProperty(AudioContext.prototype, '_baseLatency', { value: A.baseLatency });
  Object.defineProperty(AudioContext.prototype, '_outputLatency', { value: A.outputLatency });
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
    def(inst, '_numberOfChannels', a[0]);
    def(inst, '_length', a[1]);
  });
  Object.setPrototypeOf(OfflineAudioContext.prototype, Base.prototype);
  slot(OfflineAudioContext.prototype, 'length');
  Object.defineProperty(OfflineAudioContext.prototype, 'oncomplete', {
    get: brand(function () {
      return Object.prototype.hasOwnProperty.call(this, '_on_complete') ? this._on_complete : null;
    }, 'get oncomplete'),
    set: brand(function (fn) { def(this, '_on_complete', typeof fn === 'function' ? fn : null); }, 'set oncomplete'),
    enumerable: true, configurable: true,
  });
  OfflineAudioContext.prototype.suspend = brand(function () { return Promise.resolve(undefined); }, 'suspend');

  // OfflineAudioCompletionEvent: an Event subtype carrying the rendered buffer,
  // so `ctx.oncomplete = e => e.renderedBuffer` (the fingerprintjs shape) works.
  var CompletionEvent = klass('OfflineAudioCompletionEvent', function (inst, args) {
    var o = args[1] || {};
    def(inst, '_type', String(args[0]));
    def(inst, '_renderedBuffer', o.renderedBuffer === undefined ? null : o.renderedBuffer);
  });
  Object.setPrototypeOf(CompletionEvent.prototype, g.Event.prototype);
  slot(CompletionEvent.prototype, 'type');
  slot(CompletionEvent.prototype, 'renderedBuffer');

  // startRendering(): the rendered buffer's digest is FROZEN to the graph built so
  // far, so repeat calls and independent invocations agree. Resolves the promise
  // AND fires oncomplete on the microtask queue (both Firefox delivery paths),
  // which the run driver drains before it reports the run settled.
  OfflineAudioContext.prototype.startRendering = brand(function () {
    var buf = makeBuffer(this._numberOfChannels, this._length, this._sampleRate, true, SEED, this._d);
    def(this, '_currentTime', this._sampleRate ? this._length / this._sampleRate : 0);
    def(this, '_state', 'closed');
    var self = this;
    Promise.resolve().then(function () {
      var cb = self._on_complete;
      if (typeof cb === 'function') cb(new CompletionEvent('complete', { renderedBuffer: buf }));
    });
    return Promise.resolve(buf);
  }, 'startRendering');
})(globalThis);
