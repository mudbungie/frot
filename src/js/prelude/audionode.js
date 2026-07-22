// The Web Audio node zoo + AudioParam + AudioBuffer for the fingerprint
// masquerade (bl-8733, identity.md §10/§11, js.md §7). Firefox exposes the whole
// AudioNode hierarchy; a null surface is a louder tell than a costume (§10), so
// frot returns branded, Firefox-shaped nodes. Every node/param mutation folds
// into its context's graph digest (ctx._fold), so the rendered buffer (audio.js)
// is a DETERMINISTIC function of the graph — the same graph hashes identically
// every invocation, a changed one diverges, never random. Split from audio.js
// purely to keep both files under the size cap; the contexts reach this through
// the `__frot_audio_node` (node factory) and `__frot_audio_buffer` helpers.
// Runs after audiobuf.js (needs __frot_audio_render/_class/_mix) and brand.js.
// AudioParam min/max are the generic float range and analyser data methods are
// folding no-ops (offline persona) — declared §11 realism residuals. No syscall.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var iface = g.__frot_iface;
  var klass = g.__frot_audio_class;
  var render = g.__frot_audio_render;
  var FLT = 3.4028234663852886e38; // generic AudioParam min/max (Float32 max).

  function def(o, k, v, e) {
    Object.defineProperty(o, k, { value: v, configurable: true, writable: true, enumerable: !!e });
  }
  function ro(o, k, v) {
    Object.defineProperty(o, k, { value: v, configurable: true, enumerable: true });
  }
  function argstr(a) {
    var parts = [];
    for (var i = 0; i < a.length; i++) parts.push(String(a[i]));
    return parts.join(',');
  }
  function domError(name, message) {
    var e = new Error(message);
    e.name = name;
    return e;
  }

  // --- AudioParam ------------------------------------------------------------
  // Not constructable (Firefox: Illegal). `value` is read/write and folds each
  // write into the context digest (an automation call does too), so the graph a
  // fingerprinter builds — oscillator.frequency.value, the compressor params —
  // deterministically drives the rendered output. defaultValue/minValue/maxValue
  // read fixed slots (min/max the generic float range, a declared residual).
  var AudioParam = iface('AudioParam', {});
  ['defaultValue', 'minValue', 'maxValue', 'automationRate'].forEach(function (k) {
    Object.defineProperty(AudioParam.prototype, k, {
      get: brand(function () { return this['_' + k]; }, 'get ' + k),
      enumerable: true, configurable: true,
    });
  });
  Object.defineProperty(AudioParam.prototype, 'value', {
    get: brand(function () { return this._value; }, 'get value'),
    set: brand(function (v) {
      this._value = +v;
      this._ctx._fold(this._name + '=' + this._value);
    }, 'set value'),
    enumerable: true, configurable: true,
  });
  ['setValueAtTime', 'linearRampToValueAtTime', 'exponentialRampToValueAtTime',
    'setTargetAtTime', 'setValueCurveAtTime', 'cancelScheduledValues',
    'cancelAndHoldAtTime'].forEach(function (m) {
    AudioParam.prototype[m] = brand(function () {
      this._ctx._fold(this._name + '.' + m + '(' + argstr(arguments) + ')');
      return this;
    }, m);
  });
  function param(ctx, name, dflt) {
    var p = Object.create(AudioParam.prototype);
    def(p, '_ctx', ctx);
    def(p, '_name', name);
    def(p, '_value', dflt);
    def(p, '_defaultValue', dflt);
    def(p, '_minValue', -FLT);
    def(p, '_maxValue', FLT);
    def(p, '_automationRate', 'a-rate');
    return p;
  }

  // --- AudioNode base --------------------------------------------------------
  // Not constructable (Firefox: Illegal); the concrete nodes subclass it. The
  // structural props are prototype accessors reading per-instance slots; connect
  // returns its destination (so `osc.connect(comp).connect(dest)` chains, exactly
  // as Web Audio does) and folds the edge into the digest.
  var AudioNode = iface('AudioNode', {});
  ['context', 'numberOfInputs', 'numberOfOutputs'].forEach(function (k) {
    Object.defineProperty(AudioNode.prototype, k, {
      get: brand(function () { return this['_' + k]; }, 'get ' + k),
      enumerable: true, configurable: true,
    });
  });
  ['channelCount', 'channelCountMode', 'channelInterpretation'].forEach(function (k) {
    Object.defineProperty(AudioNode.prototype, k, {
      get: brand(function () { return this['_' + k]; }, 'get ' + k),
      set: brand(function (v) { def(this, '_' + k, v); }, 'set ' + k),
      enumerable: true, configurable: true,
    });
  });
  AudioNode.prototype.connect = brand(function (dest) {
    this._context._fold('connect(' + (dest && dest[Symbol.toStringTag]) + ')');
    return dest;
  }, 'connect');
  AudioNode.prototype.disconnect = brand(function () {
    this._context._fold('disconnect(' + argstr(arguments) + ')');
  }, 'disconnect');

  // --- The node specifications (data-driven; io = [inputs, outputs]) ----------
  // Firefox-shaped defaults from the Web Audio spec. `params` are AudioParams,
  // `data` settable scalar/enum props (fold on write), `ro` read-only props (a
  // function is resolved against the context, e.g. destination.maxChannelCount).
  var NODES = {
    OscillatorNode: { io: [0, 1], params: { frequency: 440, detune: 0 },
      data: { type: 'sine' }, methods: ['start', 'stop', 'setPeriodicWave'] },
    DynamicsCompressorNode: { io: [1, 1],
      params: { threshold: -24, knee: 30, ratio: 12, attack: 0.003, release: 0.25 },
      ro: { reduction: 0 } },
    GainNode: { io: [1, 1], params: { gain: 1 } },
    BiquadFilterNode: { io: [1, 1],
      params: { frequency: 350, detune: 0, Q: 1, gain: 0 },
      data: { type: 'lowpass' }, methods: ['getFrequencyResponse'] },
    AnalyserNode: { io: [1, 1],
      data: { fftSize: 2048, minDecibels: -100, maxDecibels: -30, smoothingTimeConstant: 0.8 },
      ro: { frequencyBinCount: 1024 },
      methods: ['getFloatFrequencyData', 'getByteFrequencyData',
        'getFloatTimeDomainData', 'getByteTimeDomainData'] },
    AudioBufferSourceNode: { io: [0, 1], params: { playbackRate: 1, detune: 0 },
      data: { loop: false, loopStart: 0, loopEnd: 0 }, methods: ['start', 'stop'] },
    AudioDestinationNode: { io: [1, 0], ro: { maxChannelCount: function (c) { return c._maxChannels; } } },
  };

  // Build one branded, constructable node class subclassing AudioNode. `new
  // OscillatorNode(context, options)` and the createX factory both flow through
  // initNode, so a page reaches an identical instance either way.
  var CLASSES = {};
  Object.keys(NODES).forEach(function (name) {
    var spec = NODES[name];
    var C = klass(name, function (inst, args) { initNode(inst, name, args[0], args[1]); });
    Object.setPrototypeOf(C.prototype, AudioNode.prototype);
    (spec.methods || []).forEach(function (m) {
      C.prototype[m] = brand(function () {
        this._context._fold(name + '.' + m + '(' + argstr(arguments) + ')');
      }, m);
    });
    CLASSES[name] = C;
  });

  function dataProp(inst, ctx, tag, key, dflt) {
    var slot = '_dp_' + key;
    def(inst, slot, dflt);
    Object.defineProperty(inst, key, {
      get: brand(function () { return this[slot]; }, 'get ' + key),
      set: brand(function (v) { this[slot] = v; ctx._fold(tag + '=' + String(v)); }, 'set ' + key),
      enumerable: true, configurable: true,
    });
  }
  function initNode(inst, name, ctx, options) {
    var spec = NODES[name];
    def(inst, '_context', ctx);
    def(inst, '_numberOfInputs', spec.io[0]);
    def(inst, '_numberOfOutputs', spec.io[1]);
    def(inst, '_channelCount', 2);
    def(inst, '_channelCountMode', 'max');
    def(inst, '_channelInterpretation', 'speakers');
    Object.keys(spec.params || {}).forEach(function (pn) {
      ro(inst, pn, param(ctx, name + '.' + pn, spec.params[pn]));
    });
    Object.keys(spec.data || {}).forEach(function (dn) {
      dataProp(inst, ctx, name + '.' + dn, dn, spec.data[dn]);
    });
    Object.keys(spec.ro || {}).forEach(function (rn) {
      var v = spec.ro[rn];
      ro(inst, rn, typeof v === 'function' ? v(ctx) : v);
    });
    if (options && typeof options === 'object') {
      Object.keys(options).forEach(function (ok) {
        if (ok in inst) inst[ok] = options[ok];
      });
    }
    ctx._fold('create ' + name);
    return inst;
  }
  function makeNode(name, ctx, options) {
    return initNode(Object.create(CLASSES[name].prototype), name, ctx, options);
  }

  // --- AudioBuffer -----------------------------------------------------------
  // Constructable and factory-built. A RENDERED buffer's channels are the
  // deterministic sample expansion (audiobuf.js); a plain constructed buffer is
  // silent zeros a page may fill via copyToChannel — exactly like a real fresh
  // AudioBuffer, mirroring canvas.js's never-drawn-is-transparent invariant.
  var AudioBuffer = klass('AudioBuffer', function (inst, args) {
    var o = args[0] || {};
    if (typeof o !== 'object' || !('length' in o) || !('sampleRate' in o)) {
      throw new TypeError('AudioBuffer constructor requires length and sampleRate');
    }
    shapeBuffer(inst, o.numberOfChannels === undefined ? 1 : o.numberOfChannels >>> 0,
      o.length >>> 0, +o.sampleRate, false, 0, 0);
  });
  ['sampleRate', 'length', 'numberOfChannels', 'duration'].forEach(function (k) {
    Object.defineProperty(AudioBuffer.prototype, k, {
      get: brand(function () { return this['_' + k]; }, 'get ' + k),
      enumerable: true, configurable: true,
    });
  });
  function shapeBuffer(b, nc, len, sr, rendered, seed, digest) {
    def(b, '_numberOfChannels', nc);
    def(b, '_length', len);
    def(b, '_sampleRate', sr);
    def(b, '_duration', sr ? len / sr : 0);
    def(b, '_rendered', rendered);
    def(b, '_seed', seed);
    def(b, '_digest', digest);
    def(b, '_chan', []);
    return b;
  }
  function channel(b, ch) {
    ch = ch >>> 0;
    if (ch >= b._numberOfChannels) {
      throw domError('IndexSizeError', 'Channel index is out of range.');
    }
    if (!b._chan[ch]) {
      b._chan[ch] = b._rendered
        ? render(b._seed, b._digest, b._length, ch)
        : new Float32Array(b._length);
    }
    return b._chan[ch];
  }
  AudioBuffer.prototype.getChannelData = brand(function (ch) { return channel(this, ch); }, 'getChannelData');
  AudioBuffer.prototype.copyFromChannel = brand(function (dest, ch, start) {
    var src = channel(this, ch);
    var off = start >>> 0;
    for (var i = 0; i < dest.length && off + i < src.length; i++) dest[i] = src[off + i];
  }, 'copyFromChannel');
  AudioBuffer.prototype.copyToChannel = brand(function (src, ch, start) {
    var dst = channel(this, ch);
    var off = start >>> 0;
    for (var i = 0; i < src.length && off + i < dst.length; i++) dst[off + i] = src[i];
  }, 'copyToChannel');
  function makeBuffer(nc, len, sr, rendered, seed, digest) {
    return shapeBuffer(Object.create(AudioBuffer.prototype), nc, len, sr, rendered, seed, digest);
  }

  [['__frot_audio_node', makeNode], ['__frot_audio_buffer', makeBuffer]].forEach(function (p) {
    brand(p[1], p[0]);
    Object.defineProperty(g, p[0], { value: p[1], configurable: true, writable: true });
  });
})(globalThis);
