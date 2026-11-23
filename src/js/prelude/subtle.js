// crypto.subtle — a REAL SubtleCrypto, not a costume (bl-0647, bl-db8f; ruling
// and every measurement in identity.md §3.19, residuals in identity.md §11). A
// page awaits and consumes what these methods return — AWS WAF's challenge
// encrypts its answer with one (challenge.md §11.4) — so what is implemented
// computes the true value through the __frot_subtle_* syscalls (the TLS stack's
// own crypto provider), and what is not implemented REJECTS with the
// `NotSupportedError` Firefox gives for an algorithm it does not know. Never a
// made-up result, never a promise that hangs.
//
// Implemented: digest SHA-1/256/384/512; AES-CBC, AES-CTR and AES-GCM (any iv,
// seven tag lengths) encrypt/decrypt; AES-KW; HMAC sign/verify; PBKDF2 and HKDF
// deriveBits/deriveKey; wrapKey/unwrapKey under the four AES modes;
// importKey/exportKey (`raw`, `jwk`) and generateKey for all of them; and
// (bl-0cae, subtlepk.js) ECDSA, ECDH, Ed25519 and X25519 — key pairs, `spki`,
// `pkcs8`, sign/verify, deriveBits/deriveKey. RSA* rejects — the declared
// residual, bl-2763.
//
// Shapes, member order, `.length`s, usage order, JWK key order and every error
// name, message and ORDER below were READ off Firefox 153.0esr. `[SecureContext]`:
// on an insecure page none of this exists. Runs after subtlekey.js and
// subtlealg.js. Errors are brand.js's DOMException-shaped ones.
(function (g) {
  'use strict';
  var k = g.__frot_subtle;
  if (!k) return; // insecure context: subtlekey.js defined nothing
  delete g.__frot_subtle;
  var brand = g.__frot_brand;
  var Promise = g.Promise; // the engine's, whatever a page later assigns
  var fail = k.fail, normalize = k.normalize, keyed = k.keyed, isBuffer = k.isBuffer;
  var copy = k.copy, buf = k.buf, sequence = k.sequence, usageList = k.usageList;
  var makeKey = k.makeKey, keyArg = k.keyArg, has = k.has, known = k.known, run = k.run;
  var HASH = k.HASH, FORMATS = k.FORMATS, ALG = k.ALG;
  // The names each method takes (anything else: NotSupportedError). The ones
  // frot has no key for fail the key comparison instead, as on the binary.
  var CIPHERS = ['AES-CBC', 'AES-CTR', 'AES-GCM', 'RSA-OAEP'];
  var WRAPS = CIPHERS.concat('AES-KW');
  var SIGNS = ['RSASSA-PKCS1-v1_5', 'RSA-PSS', 'ECDSA', 'Ed25519', 'HMAC'];
  var DERIVES = ['ECDH', 'X25519', 'HKDF', 'PBKDF2'];

  function ascii(str) {
    return Uint8Array.from(str, function (c) {
      return c.charCodeAt(0);
    });
  }
  function cipher(name, open) {
    return function (algorithm, key, data) {
      var s = keyArg(key, name + ': Argument 2');
      var bytes = buf(data, name + ': Argument 3');
      has(s, name);
      return run(normalize(algorithm, CIPHERS), open, s, bytes).buffer;
    };
  }
  // The algorithm a key that may `usage` signs under — the key's own.
  function signer(algorithm, s, usage) {
    has(s, usage);
    var n = normalize(algorithm, SIGNS);
    if (n.name !== s.algorithm.name) throw fail('InvalidAccessError');
    return n;
  }
  function mac(s, data) {
    return g.__frot_subtle_hmac(s.algorithm.hash.name, s.bytes, data);
  }
  // A key from its bytes or its JWK. Firefox reads the DATA, not the format, to
  // tell them apart: a buffer is raw key bytes even under `jwk`, and an object
  // is a JWK — which `raw` then refuses.
  function imported(format, keyData, n, extractable, list) {
    var impl = n.impl, raw = isBuffer(keyData), jwk = raw ? {} : keyData;
    var extra = impl.params(n.p);
    var use = impl.usagesFirst && usageList(list, impl.usages);
    if (jwk.kty === undefined && !raw) throw fail('DataError');
    if (format === 'raw' && !raw) throw fail('SyntaxError');
    if (jwk.ext === false && extractable) throw fail('DataError');
    var bytes = raw ? copy(keyData) : k.unb64url(jwk.k);
    var alg = impl.imported(n.p, bytes, extra);
    use = use || usageList(list, impl.usages);
    var ops = jwk.key_ops;
    if ((jwk.alg !== undefined && jwk.alg !== impl.jwk(alg))
      || (jwk.use !== undefined && jwk.use !== impl.use)
      || (ops !== undefined && !(Array.isArray(ops) && use.every(function (u) {
        return ops.indexOf(u) >= 0;
      })))) throw fail('DataError');
    return makeKey(alg, extractable, use, bytes);
  }
  // A key's bytes, or its JWK (members in Firefox's order; no `alg` where the
  // algorithm has none).
  function exported(format, s) {
    if (s.pk) return k.pkExport(format, s);
    if (format === 'raw') return copy(s.bytes);
    if (format !== 'jwk') throw fail('NotSupportedError');
    var alg = ALG[s.algorithm.name].jwk(s.algorithm);
    return Object.assign(alg ? { alg: alg } : {}, {
      ext: true, k: k.b64url(s.bytes), key_ops: s.usages.slice(), kty: 'oct',
    });
  }

  var OPS = {
    encrypt: [3, cipher('encrypt', false)],
    decrypt: [3, cipher('decrypt', true)],
    sign: [3, function (algorithm, key, data) {
      var s = keyArg(key, 'sign: Argument 2');
      var bytes = buf(data, 'sign: Argument 3'), n = signer(algorithm, s, 'sign');
      return (s.pk ? k.pkSign(n, s, bytes) : mac(s, bytes)).buffer;
    }],
    verify: [4, function (algorithm, key, signature, data) {
      var s = keyArg(key, 'verify: Argument 2');
      var sig = buf(signature, 'verify: Argument 3');
      var bytes = buf(data, 'verify: Argument 4'), n = signer(algorithm, s, 'verify');
      if (s.pk) return k.pkVerify(n, s, sig, bytes);
      var want = mac(s, bytes);
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
      if (n.impl.pk) return k.pkGenerate(n, extractable, list);
      var made = n.impl.generated(n.p, n.impl.params(n.p));
      var use = usageList(list, n.impl.usages);
      return makeKey(made[0], extractable, use, Uint8Array.from(g.__frot_random_bytes(made[1])));
    }],
    // The derived key's length is what generateKey would give its type; an
    // unknown type is a SyntaxError here (measured), not NotSupportedError.
    deriveKey: [5, function (algorithm, key, type, extractable, usages) {
      var s = keyArg(key, 'deriveKey: Argument 2');
      var list = sequence(usages, 'deriveKey: Argument 5');
      has(s, 'deriveKey');
      known(list);
      var n = normalize(algorithm, DERIVES), t;
      try {
        normalize(type, k.KEYED.concat(Object.keys(HASH)));
      } catch (e) {
        throw fail('SyntaxError');
      }
      t = keyed(type);
      // An agreement takes a derivation key's whole secret (measured: HKDF).
      var bits = t.impl.kdf && ALG[n.name].pk ? null
        : t.impl.generated(t.p, t.impl.params(t.p))[0].length;
      return imported('raw', k.derive(n, s, bits), t, extractable, list);
    }],
    deriveBits: [2, function (algorithm, key, length) {
      var s = has(keyArg(key, 'deriveBits: Argument 2'), 'deriveBits');
      return k.derive(normalize(algorithm, DERIVES), s, length).buffer;
    }],
    importKey: [5, function (format, keyData, algorithm, extractable, usages) {
      if (keyData === null || typeof keyData !== 'object') {
        throw new TypeError('SubtleCrypto.importKey: Argument 2 is not an object.');
      }
      var list = sequence(usages, 'importKey: Argument 5');
      var n = keyed(algorithm);
      if (FORMATS.indexOf(format) < 0) throw fail('SyntaxError');
      if (n.impl.pk) return k.pkImport(format, keyData, n, extractable, list);
      if (format !== 'raw' && format !== 'jwk') throw fail('NotSupportedError');
      return imported(format, keyData, n, extractable, list);
    }],
    exportKey: [2, function (format, key) {
      var s = keyArg(key, 'exportKey: Argument 2');
      if (FORMATS.indexOf(format) < 0) throw fail('SyntaxError');
      if (!s.extractable) throw fail('InvalidAccessError');
      if (ALG[s.algorithm.name].sealed) throw fail('NotSupportedError');
      var out = exported(format, s);
      return out instanceof Uint8Array ? out.buffer : out;
    }],
    // export, then encrypt: the format is judged before the wrapping key is
    // compared with the algorithm, and a JWK travels as its JSON text.
    wrapKey: [4, function (format, key, wrappingKey, wrapAlgorithm) {
      var s = keyArg(key, 'wrapKey: Argument 2');
      var w = keyArg(wrappingKey, 'wrapKey: Argument 3');
      if (FORMATS.indexOf(format) < 0) throw fail('SyntaxError');
      has(w, 'wrapKey');
      if (!s.extractable) throw fail('InvalidAccessError');
      var n = normalize(wrapAlgorithm, WRAPS);
      var out = exported(format, s);
      return run(n, false, w, out instanceof Uint8Array ? out : ascii(JSON.stringify(out))).buffer;
    }],
    // decrypt, then import — every importKey rule applies to what comes out,
    // except that an unusable format is NotSupportedError here.
    unwrapKey: [7, function (format, wrappedKey, unwrappingKey, unwrapAlgorithm, type,
      extractable, usages) {
      var bytes = buf(wrappedKey, 'unwrapKey: Argument 2');
      var w = keyArg(unwrappingKey, 'unwrapKey: Argument 3');
      var list = sequence(usages, 'unwrapKey: Argument 7');
      has(w, 'unwrapKey');
      known(list);
      var t = keyed(type);
      if (t.name === 'PBKDF2') throw fail('NotSupportedError'); // measured; HKDF is fine
      var data = run(normalize(unwrapAlgorithm, WRAPS), true, w, bytes);
      if (format === 'jwk') {
        try {
          data = Object(JSON.parse(String.fromCharCode.apply(null, data)));
        } catch (e) {
          data = {};
        }
      }
      if (t.impl.pk) return k.pkImport(format, data, t, extractable, list);
      if (format !== 'raw' && format !== 'jwk') throw fail('NotSupportedError');
      return imported(format, data, t, extractable, list);
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
