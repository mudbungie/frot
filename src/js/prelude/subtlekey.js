// The WebCrypto toolkit under subtle.js (bl-0647): the error table, algorithm
// normalization, BufferSource and usage handling, base64url, the `CryptoKey`
// interface, and the two keyed algorithms' rules. Split from subtle.js under the
// 300-line cap on the seam "what a key IS" / "what the twelve methods DO"; the
// header there carries the scope, the ruling and the provenance (identity.md
// §3.19 — every name, order and message here was read off Firefox 153.0esr).
// Hands subtle.js one non-enumerable `__frot_subtle` object, which that module
// removes. `[SecureContext]`: on an insecure page neither module defines
// anything. Runs after crypto.js.
(function (g) {
  'use strict';
  if (!g.__frot_secure_context()) return;
  var slots = g.__frot_slots;
  var MSG = {
    NotSupportedError: 'Operation is not supported',
    SyntaxError: 'An invalid or illegal string was specified',
    DataError: 'Data provided to an operation does not meet requirements',
    InvalidAccessError: 'A parameter or an operation is not supported by the underlying object',
    OperationError: 'The operation failed for an operation-specific reason',
  };
  function fail(name) {
    return g.__frot_domerror(name, MSG[name]);
  }

  // Block size in bits (HMAC's generated key length) and the JWK `alg`.
  var HASH = {
    'SHA-1': [512, 'HS1'], 'SHA-256': [512, 'HS256'],
    'SHA-384': [1024, 'HS384'], 'SHA-512': [1024, 'HS512'],
  };
  // Every non-digest algorithm 153.0esr recognises. A name outside this list and
  // HASH is unknown to Firefox too.
  // (A digest name is as unsupported as an unknown one wherever a key is asked for.)
  var KEYED = ['RSASSA-PKCS1-v1_5', 'RSA-PSS', 'RSA-OAEP', 'ECDSA', 'ECDH', 'Ed25519',
    'X25519', 'AES-CTR', 'AES-CBC', 'AES-GCM', 'AES-KW', 'HMAC', 'HKDF', 'PBKDF2'];
  // WebIDL's KeyUsage order — the order `key.usages` reports, whatever was asked.
  var USAGES = ['encrypt', 'decrypt', 'sign', 'verify', 'deriveKey', 'deriveBits',
    'wrapKey', 'unwrapKey'];
  var FORMATS = ['raw', 'pkcs8', 'spki', 'jwk'];

  // An AlgorithmIdentifier -> { name: canonical, p: its dictionary }. `names` is
  // what this operation accepts; anything else is NotSupportedError.
  function normalize(alg, names) {
    var dict = alg !== null && typeof alg === 'object';
    if (dict && alg.name === undefined) throw new TypeError('The operation could not be performed.');
    var want = String(dict ? alg.name : alg).toLowerCase();
    var name = names.filter(function (n) {
      return n.toLowerCase() === want;
    })[0];
    if (name === undefined) throw fail('NotSupportedError');
    return { name: name, p: dict ? alg : {} };
  }
  function hashName(h) {
    try {
      return normalize(h, Object.keys(HASH)).name;
    } catch (e) {
      throw fail('SyntaxError');
    }
  }

  function isBuffer(v) {
    return v instanceof ArrayBuffer || ArrayBuffer.isView(v);
  }
  // A private copy of a BufferSource's bytes.
  function copy(v) {
    return v instanceof ArrayBuffer ? new Uint8Array(v.slice(0))
      : new Uint8Array(v.buffer.slice(v.byteOffset, v.byteOffset + v.byteLength));
  }
  function buf(v, where) {
    if (!isBuffer(v)) {
      throw new TypeError('SubtleCrypto.' + where
        + ' could not be converted to any of: ArrayBufferView, ArrayBuffer.');
    }
    return copy(v);
  }
  function sequence(v, where) {
    if (v === null || typeof v !== 'object' || typeof v[Symbol.iterator] !== 'function') {
      throw new TypeError('SubtleCrypto.' + where + " can't be converted to a sequence.");
    }
    return Array.from(v, String);
  }
  function usageList(list, allowed) {
    var bad = list.some(function (u) {
      return allowed.indexOf(u) < 0;
    });
    if (bad || !list.length) throw fail('SyntaxError');
    return USAGES.filter(function (u) {
      return list.indexOf(u) >= 0;
    });
  }

  var B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_';
  function b64url(bytes) {
    var s = '';
    for (var i = 0; i < bytes.length; i += 3) {
      var n = (bytes[i] << 16) | ((bytes[i + 1] || 0) << 8) | (bytes[i + 2] || 0);
      s += B64[n >> 18] + B64[(n >> 12) & 63]
        + (i + 1 < bytes.length ? B64[(n >> 6) & 63] : '')
        + (i + 2 < bytes.length ? B64[n & 63] : '');
    }
    return s;
  }
  function unb64url(str) {
    if (typeof str !== 'string' || /[^A-Za-z0-9_-]/.test(str) || str.length % 4 === 1) {
      throw fail('DataError');
    }
    var out = [], acc = 0, bits = 0;
    for (var i = 0; i < str.length; i++) {
      acc = ((acc << 6) | B64.indexOf(str[i])) & 0xffff;
      bits += 6;
      if (bits >= 8) {
        bits -= 8;
        out.push((acc >> bits) & 255);
      }
    }
    return new Uint8Array(out);
  }

  // --- CryptoKey ---------------------------------------------------------------
  // The four attributes read the instance's backing slots; the key bytes sit in
  // a fifth slot no attribute exposes, so the instance owns nothing (measured).
  var CryptoKey = g.__frot_iface('CryptoKey', ['type', 'extractable', 'algorithm', 'usages']);
  function makeKey(algorithm, extractable, usages, bytes) {
    var key = Object.create(CryptoKey.prototype);
    Object.assign(slots(key), {
      type: 'secret', extractable: !!extractable, algorithm: algorithm, usages: usages,
      bytes: bytes,
    });
    return key;
  }
  function keyArg(key, where) {
    var s = slots(key);
    if (!s.bytes) {
      throw new TypeError('SubtleCrypto.' + where + ' does not implement interface CryptoKey.');
    }
    return s;
  }
  // The key must be of the named algorithm and carry the usage.
  function holding(key, where, name, usage) {
    var s = keyArg(key, where);
    if (s.algorithm.name !== name || s.usages.indexOf(usage) < 0) throw fail('InvalidAccessError');
    return s;
  }

  // --- The two keyed algorithms -------------------------------------------------
  // `params` validates the dictionary (SyntaxError); `imported` judges key bytes
  // (DataError) and returns `key.algorithm`; `generated` returns [algorithm,
  // byte count]. `usagesFirst` is the measured order of the usage and data checks.
  var ALG = {
    'AES-GCM': {
      usages: ['encrypt', 'decrypt', 'wrapKey', 'unwrapKey'], use: 'enc', usagesFirst: true,
      params: function () {},
      imported: function (p, bytes) {
        if ([16, 24, 32].indexOf(bytes.length) < 0) throw fail('DataError');
        return { name: 'AES-GCM', length: bytes.length * 8 };
      },
      generated: function (p) {
        if (p.length === undefined) throw fail('SyntaxError');
        var bits = Number(p.length);
        if ([128, 192, 256].indexOf(bits) < 0) throw fail('OperationError');
        return [{ name: 'AES-GCM', length: bits }, bits / 8];
      },
      jwk: function (a) {
        return 'A' + a.length + 'GCM';
      },
    },
    HMAC: {
      usages: ['sign', 'verify'], use: 'sig', usagesFirst: false,
      params: function (p) {
        return hashName(p.hash);
      },
      imported: function (p, bytes, hash) {
        if (!bytes.length) throw fail('DataError');
        return { name: 'HMAC', hash: { name: hash }, length: bytes.length * 8 };
      },
      generated: function (p, hash) {
        var bits = p.length === undefined ? HASH[hash][0] : Number(p.length);
        if (!bits) throw fail('DataError');
        return [{ name: 'HMAC', hash: { name: hash }, length: bits }, Math.floor(bits / 8)];
      },
      jwk: function (a) {
        return HASH[a.hash.name][1];
      },
    },
  };
  function keyed(alg) {
    var n = normalize(alg, KEYED);
    n.impl = ALG[n.name];
    if (!n.impl) throw fail('NotSupportedError');
    return n;
  }


  Object.defineProperty(g, '__frot_subtle', {
    configurable: true,
    value: {
      fail: fail, normalize: normalize, keyed: keyed, isBuffer: isBuffer, copy: copy, buf: buf,
      sequence: sequence, usageList: usageList, b64url: b64url, unb64url: unb64url,
      makeKey: makeKey, keyArg: keyArg, holding: holding,
      HASH: HASH, KEYED: KEYED, FORMATS: FORMATS, ALG: ALG,
    },
  });
})(globalThis);
