// crypto.subtle — a REAL SubtleCrypto, not a costume (bl-0647; ruling and every
// measurement in identity.md §3.19, residuals in identity.md §11). A page awaits
// and consumes what these methods return — AWS WAF's challenge encrypts its
// answer with one (challenge.md §11.4) — so what is implemented computes the true
// value through the __frot_subtle_* syscalls (the TLS stack's own crypto
// provider), and what is not implemented REJECTS with the `NotSupportedError`
// Firefox gives for an algorithm it does not know. Never a made-up result, never
// a promise that hangs.
//
// Implemented: digest SHA-1/256/384/512; AES-GCM (128/192/256-bit keys, 96-bit
// iv, 128-bit tag) importKey/exportKey/generateKey/encrypt/decrypt; HMAC over the
// four digests importKey/exportKey/generateKey/sign/verify; `raw` and `jwk`.
// Everything else Firefox 153 has rejects — the declared residual.
//
// Shapes, member order, `.length`s, usage order, JWK key order and every error
// name and message below were READ off Firefox 153.0esr. `[SecureContext]`: on
// an insecure page none of this exists. Runs after crypto.js (extends
// Crypto.prototype). Errors are brand.js's DOMException-shaped ones.
(function (g) {
  'use strict';
  var k = g.__frot_subtle;
  if (!k) return; // insecure context: subtlekey.js defined nothing
  delete g.__frot_subtle;
  var brand = g.__frot_brand;
  var Promise = g.Promise; // the engine's, whatever a page later assigns
  var fail = k.fail, normalize = k.normalize, keyed = k.keyed, isBuffer = k.isBuffer;
  var copy = k.copy, buf = k.buf, sequence = k.sequence, usageList = k.usageList;
  var makeKey = k.makeKey, keyArg = k.keyArg, holding = k.holding;
  var HASH = k.HASH, KEYED = k.KEYED, FORMATS = k.FORMATS, ALG = k.ALG;

  function gcm(open, s, p, data) {
    var tag = p.tagLength === undefined ? 128 : Number(p.tagLength);
    var aad = p.additionalData === undefined ? new Uint8Array(0) : p.additionalData;
    if (!isBuffer(p.iv) || !p.iv.byteLength || !isBuffer(aad)
      || [32, 64, 96, 104, 112, 120, 128].indexOf(tag) < 0) throw fail('OperationError');
    // Firefox takes any iv length and the shorter tags; frot's provider is
    // 96/128 only. Refused honestly rather than answered wrongly (residual).
    if (p.iv.byteLength !== 12 || tag !== 128) throw fail('NotSupportedError');
    var out = g.__frot_subtle_gcm(open, s.bytes, copy(p.iv), copy(aad), data);
    if (!out) throw fail('OperationError');
    return out.buffer;
  }
  function cipher(name, open) {
    return function (algorithm, key, data) {
      keyArg(key, name + ': Argument 2');
      var bytes = buf(data, name + ': Argument 3');
      var n = normalize(algorithm, KEYED);
      return gcm(open, holding(key, '', n.name, name), n.p, bytes);
    };
  }
  function mac(s, data) {
    return g.__frot_subtle_hmac(s.algorithm.hash.name, s.bytes, data);
  }
  function fromJwk(jwk, extractable) {
    if (jwk.kty === undefined || (jwk.ext === false && extractable)) throw fail('DataError');
    return k.unb64url(jwk.k);
  }

  var OPS = {
    encrypt: [3, cipher('encrypt', false)],
    decrypt: [3, cipher('decrypt', true)],
    sign: [3, function (algorithm, key, data) {
      keyArg(key, 'sign: Argument 2');
      var bytes = buf(data, 'sign: Argument 3');
      return mac(holding(key, '', normalize(algorithm, KEYED).name, 'sign'), bytes).buffer;
    }],
    verify: [4, function (algorithm, key, signature, data) {
      keyArg(key, 'verify: Argument 2');
      var sig = buf(signature, 'verify: Argument 3');
      var bytes = buf(data, 'verify: Argument 4');
      var want = mac(holding(key, '', normalize(algorithm, KEYED).name, 'verify'), bytes);
      var diff = sig.length ^ want.length;
      for (var i = 0; i < want.length; i++) diff |= sig[i] ^ want[i];
      return diff === 0;
    }],
    digest: [2, function (algorithm, data) {
      var bytes = buf(data, 'digest: Argument 2');
      return g.__frot_subtle_digest(normalize(algorithm, Object.keys(HASH)).name, bytes).buffer;
    }],
    generateKey: [3, function (algorithm, extractable, usages) {
      var list = sequence(usages, 'generateKey: Argument 3');
      var n = keyed(algorithm);
      var made = n.impl.generated(n.p, n.impl.params(n.p));
      var use = usageList(list, n.impl.usages);
      return makeKey(made[0], extractable, use, Uint8Array.from(g.__frot_random_bytes(made[1])));
    }],
    // A frot key is AES-GCM or HMAC, and neither can carry a derive usage, so
    // these two answer what Firefox answers for such a key.
    deriveKey: [5, function (algorithm, key) {
      keyArg(key, 'deriveKey: Argument 2');
      throw fail('InvalidAccessError');
    }],
    deriveBits: [2, function (algorithm, key) {
      keyArg(key, 'deriveBits: Argument 2');
      throw fail('InvalidAccessError');
    }],
    importKey: [5, function (format, keyData, algorithm, extractable, usages) {
      if (keyData === null || typeof keyData !== 'object') {
        throw new TypeError('SubtleCrypto.importKey: Argument 2 is not an object.');
      }
      var list = sequence(usages, 'importKey: Argument 5');
      var n = keyed(algorithm);
      if (FORMATS.indexOf(format) < 0) throw fail('SyntaxError');
      if (format !== 'raw' && format !== 'jwk') throw fail('NotSupportedError');
      var impl = n.impl, jwk = format === 'jwk' ? keyData : {};
      var extra = impl.params(n.p);
      var use = impl.usagesFirst && usageList(list, impl.usages);
      if (format === 'raw' && !isBuffer(keyData)) throw fail('SyntaxError');
      var bytes = format === 'raw' ? copy(keyData) : fromJwk(jwk, extractable);
      var alg = impl.imported(n.p, bytes, extra);
      use = use || usageList(list, impl.usages);
      var ops = jwk.key_ops;
      if ((jwk.alg !== undefined && jwk.alg !== impl.jwk(alg))
        || (jwk.use !== undefined && jwk.use !== impl.use)
        || (ops !== undefined && !(Array.isArray(ops) && use.every(function (u) {
          return ops.indexOf(u) >= 0;
        })))) throw fail('DataError');
      return makeKey(alg, extractable, use, bytes);
    }],
    exportKey: [2, function (format, key) {
      var s = keyArg(key, 'exportKey: Argument 2');
      if (FORMATS.indexOf(format) < 0) throw fail('SyntaxError');
      if (!s.extractable) throw fail('InvalidAccessError');
      if (format === 'raw') return copy(s.bytes).buffer;
      if (format !== 'jwk') throw fail('NotSupportedError');
      return {
        alg: ALG[s.algorithm.name].jwk(s.algorithm), ext: true, k: k.b64url(s.bytes),
        key_ops: s.usages.slice(), kty: 'oct',
      };
    }],
    // Firefox wraps and unwraps with AES-GCM; frot does not yet (residual). The
    // usage check is Firefox's own, so only a key that COULD wrap is refused.
    wrapKey: [4, function (format, key, wrappingKey, wrapAlgorithm) {
      keyArg(key, 'wrapKey: Argument 2');
      keyArg(wrappingKey, 'wrapKey: Argument 3');
      holding(wrappingKey, '', normalize(wrapAlgorithm, KEYED).name, 'wrapKey');
      throw fail('NotSupportedError');
    }],
    unwrapKey: [7, function (format, wrappedKey, unwrappingKey, unwrapAlgorithm) {
      buf(wrappedKey, 'unwrapKey: Argument 2');
      keyArg(unwrappingKey, 'unwrapKey: Argument 3');
      holding(unwrappingKey, '', normalize(unwrapAlgorithm, KEYED).name, 'unwrapKey');
      throw fail('NotSupportedError');
    }],
  };

  // --- SubtleCrypto --------------------------------------------------------------
  // Every method returns a Promise, and a failure of ANY kind — a WebIDL argument
  // error included — is a rejection, never a synchronous throw (measured).
  var SubtleCrypto = g.__frot_iface('SubtleCrypto');
  var subtle = Object.create(SubtleCrypto.prototype);
  Object.keys(OPS).forEach(function (name) {
    var arity = OPS[name][0], body = OPS[name][1];
    var fn = function () {
      var self = this, args = arguments;
      return new Promise(function (resolve) {
        if (self !== subtle) {
          throw new TypeError("'" + name + "' called on an object that does not implement "
            + 'interface SubtleCrypto.');
        }
        if (args.length < arity) {
          throw new TypeError('SubtleCrypto.' + name + ': At least ' + arity
            + ' arguments required, but only ' + args.length + ' passed');
        }
        resolve(body.apply(null, args));
      });
    };
    Object.defineProperty(fn, 'length', { value: arity, configurable: true });
    SubtleCrypto.prototype[name] = brand(fn, name);
  });

  var crypto = g.crypto;
  Object.defineProperty(g.Crypto.prototype, 'subtle', {
    get: brand(function () {
      if (this !== crypto) {
        throw new TypeError("'get subtle' called on an object that does not implement "
          + 'interface Crypto.');
      }
      return subtle;
    }, 'get subtle'),
    enumerable: true,
    configurable: true,
  });
})(globalThis);
