// The Web Audio node zoo + AudioParam + AudioBuffer for the fingerprint
// masquerade (bl-8733, identity.md §10/§11, js.md §7). Firefox exposes the whole
// AudioNode hierarchy; a null surface is a louder tell than a costume (§10), so
// frot returns branded, Firefox-shaped nodes. Every node/param mutation folds
// into its context's graph digest (__frot_audio_fold), so the rendered buffer
// is a DETERMINISTIC function of the graph — the same graph hashes identically
// every invocation, a changed one diverges, never random. Split from audio.js
// purely to keep both files under the size cap; the contexts reach this through
// the `__frot_audio_node` (node factory) and `__frot_audio_buffer` helpers.
// Runs after audiobuf.js (needs __frot_audio_render/_class/_mix) and brand.js.
// AudioParam min/max are the generic float range and analyser data methods are
// folding no-ops (offline persona) — declared identity.md §11 realism residuals.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var iface = g.__frot_iface;
  var klass = g.__frot_audio_class;
  var render = g.__frot_audio_render;
  var attrs = g.__frot_ifaceattrs;
  var rwAttrs = g.__frot_rwattrs;
  var fold = g.__frot_audio_fold;
  // Node/param/buffer state lives in brand.js's one instance-state WeakMap, so a
  // page walking an oscillator sees the empty own-property list a real Gecko
  // node has (bl-3bdc, identity.md §3.16) instead of frot's backing slots.
  var slots = g.__frot_slots;
  var FLT = 3.4028234663852886e38; // generic AudioParam min/max (Float32 max).

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
  var AudioParam = iface('AudioParam', ['defaultValue', 'minValue', 'maxValue',
    'automationRate']);
  rwAttrs(AudioParam.prototype, ['value'], function () {
    var st = slots(this);
    st.value = +st.value;
    fold(st.ctx, st.name + '=' + st.value);
  });
  ['setValueAtTime', 'linearRampToValueAtTime', 'exponentialRampToValueAtTime',
    'setTargetAtTime', 'setValueCurveAtTime', 'cancelScheduledValues',
    'cancelAndHoldAtTime'].forEach(function (m) {
    AudioParam.prototype[m] = brand(function () {
      fold(slots(this).ctx, slots(this).name + '.' + m + '(' + argstr(arguments) + ')');
      return this;
    }, m);
  });
  function param(ctx, name, dflt) {
    var p = Object.create(AudioParam.prototype);
    var st = slots(p);
    st.ctx = ctx;
    st.name = name;
    st.value = dflt;
    st.defaultValue = dflt;
    st.minValue = -FLT;
    st.maxValue = FLT;
    st.automationRate = 'a-rate';
    return p;
  }

  // --- AudioNode base --------------------------------------------------------
  // Not constructable (Firefox: Illegal); the concrete nodes subclass it. The
  // structural props are prototype accessors reading per-instance slots; connect
  // returns its destination (so `osc.connect(comp).connect(dest)` chains, exactly
  // as Web Audio does) and folds the edge into the digest.
  var AudioNode = iface('AudioNode', ['context', 'numberOfInputs', 'numberOfOutputs']);
  rwAttrs(AudioNode.prototype, ['channelCount', 'channelCountMode',
    'channelInterpretation']);
  AudioNode.prototype.connect = brand(function (dest) {
    fold(slots(this).context, 'connect(' + (dest && dest[Symbol.toStringTag]) + ')');
    return dest;
  }, 'connect');
  AudioNode.prototype.disconnect = brand(function () {
    fold(slots(this).context, 'disconnect(' + argstr(arguments) + ')');
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
    AudioDestinationNode: { io: [1, 0], ro: { maxChannelCount: function (c) { return slots(c).maxChannels; } } },
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
        fold(slots(this).context, name + '.' + m + '(' + argstr(arguments) + ')');
      }, m);
    });
    // A concrete node's own members are PROTOTYPE accessors over the instance's
    // slots — `frequency`, `type`, `maxChannelCount` — not properties stamped
    // onto each instance, which is both Gecko's shape and the only shape that
    // leaves the instance's own-property list empty (bl-3bdc).
    attrs(C.prototype, Object.keys(spec.params || {}));
    rwAttrs(C.prototype, Object.keys(spec.data || {}), function (key, v) {
      fold(slots(this).context, name + '.' + key + '=' + String(v));
    });
    attrs(C.prototype, Object.keys(spec.ro || {}));
    CLASSES[name] = C;
  });

  function initNode(inst, name, ctx, options) {
    var spec = NODES[name];
    var st = slots(inst);
    st.context = ctx;
    st.numberOfInputs = spec.io[0];
    st.numberOfOutputs = spec.io[1];
    st.channelCount = 2;
    st.channelCountMode = 'max';
    st.channelInterpretation = 'speakers';
    Object.keys(spec.params || {}).forEach(function (pn) {
      st[pn] = param(ctx, name + '.' + pn, spec.params[pn]);
    });
    Object.keys(spec.data || {}).forEach(function (dn) {
      st[dn] = spec.data[dn];
    });
    Object.keys(spec.ro || {}).forEach(function (rn) {
      var v = spec.ro[rn];
      st[rn] = typeof v === 'function' ? v(ctx) : v;
    });
    if (options && typeof options === 'object') {
      Object.keys(options).forEach(function (ok) {
        if (ok in inst) inst[ok] = options[ok];
      });
    }
    fold(ctx, 'create ' + name);
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
  attrs(AudioBuffer.prototype, ['sampleRate', 'length', 'numberOfChannels', 'duration']);
  function shapeBuffer(b, nc, len, sr, rendered, seed, digest) {
    var st = slots(b);
    st.numberOfChannels = nc;
    st.length = len;
    st.sampleRate = sr;
    st.duration = sr ? len / sr : 0;
    st.rendered = rendered;
    st.seed = seed;
    st.digest = digest;
    st.chan = [];
    return b;
  }
  function channel(b, ch) {
    ch = ch >>> 0;
    var st = slots(b);
    if (ch >= st.numberOfChannels) {
      throw domError('IndexSizeError', 'Channel index is out of range.');
    }
    if (!st.chan[ch]) {
      st.chan[ch] = st.rendered
        ? render(st.seed, st.digest, st.length, ch)
        : new Float32Array(st.length);
    }
    return st.chan[ch];
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
