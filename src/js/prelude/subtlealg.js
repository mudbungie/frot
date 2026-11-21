// What each WebCrypto algorithm requires (bl-db8f): the rules a key of it obeys,
// the parameters its cipher or derivation takes, and the call into the
// __frot_subtle_* syscall that computes it. Second of the three modules — see
// subtlekey.js and subtle.js, whose header carries the scope and provenance.
// Every check, its order and its error name below was READ off Firefox 153.0esr
// (identity.md §3.19); none is the specification's. Adds `ALG`, `keyed`, `run`
// and `derive` to the `__frot_subtle` toolkit.
(function (g) {
  'use strict';
  var k = g.__frot_subtle;
  if (!k) return; // insecure context
  var fail = k.fail, isBuffer = k.isBuffer, copy = k.copy, enforce = k.enforce, HASH = k.HASH;

  // --- Keys ----------------------------------------------------------------------
  // `params` validates the dictionary (SyntaxError); `imported` judges key bytes
  // (DataError) and returns `key.algorithm`; `generated` returns [algorithm,
  // byte count] — its `length` is also what deriveKey derives for the type.
  // `usagesFirst` is the measured order of the usage and data checks; `jwk` is
  // the JWK `alg` (none for a derivation key) and `use` its `use`.
  function aes(name, suffix, usages) {
    function alg(bits, err) {
      if ([128, 192, 256].indexOf(bits) < 0) throw fail(err);
      return { name: name, length: bits };
    }
    return {
      usages: usages, use: 'enc', usagesFirst: true,
      params: function () {},
      imported: function (p, bytes) {
        return alg(bytes.length * 8, 'DataError');
      },
      generated: function (p) {
        if (p.length === undefined) throw fail('SyntaxError');
        var made = alg(Number(p.length), 'OperationError');
        return [made, made.length / 8];
      },
      jwk: function (a) {
        return 'A' + a.length + suffix;
      },
    };
  }
  // A derivation key is any bytes at all, even none; it cannot be generated,
  // and so cannot be what deriveKey derives either.
  function kdf(name, sealed) {
    return {
      usages: ['deriveKey', 'deriveBits'], usagesFirst: true, sealed: sealed,
      params: function () {},
      imported: function () {
        return { name: name };
      },
      generated: function () {
        throw fail('NotSupportedError');
      },
      jwk: function () {},
    };
  }
  var CRYPT = ['encrypt', 'decrypt', 'wrapKey', 'unwrapKey'];
  var ALG = {
    'AES-CTR': aes('AES-CTR', 'CTR', CRYPT),
    'AES-CBC': aes('AES-CBC', 'CBC', CRYPT),
    'AES-GCM': aes('AES-GCM', 'GCM', CRYPT),
    'AES-KW': aes('AES-KW', 'KW', ['wrapKey', 'unwrapKey']),
    HMAC: {
      usages: ['sign', 'verify'], use: 'sig', usagesFirst: false,
      params: function (p) {
        return k.hashName(p.hash);
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
    // exportKey refuses an HKDF key in every format, a PBKDF2 key in none.
    HKDF: kdf('HKDF', true),
    PBKDF2: kdf('PBKDF2', false),
  };
  // An algorithm a key can be OF. The names Firefox has and frot does not
  // (RSA*, ECDSA, ECDH, Ed25519, X25519) are the declared residual, bl-0cae.
  k.ALG = ALG;
  k.keyed = function (alg) {
    var n = k.normalize(alg, k.KEYED);
    n.impl = ALG[n.name];
    if (!n.impl) throw fail('NotSupportedError');
    return n;
  };

  // --- Ciphers --------------------------------------------------------------------
  // (open, key slots, dictionary, bytes) -> bytes, or nothing when the syscall
  // refuses (OperationError). A malformed dictionary is a different error in
  // each mode — measured, not a slip.
  var TAGS = [32, 64, 96, 104, 112, 120, 128];
  var CIPHER = {
    'AES-GCM': function (open, s, p, data) {
      var tag = p.tagLength === undefined ? 128 : Math.trunc(Number(p.tagLength));
      var aad = p.additionalData === undefined ? new Uint8Array(0) : p.additionalData;
      if (!isBuffer(p.iv) || !p.iv.byteLength || !isBuffer(aad) || TAGS.indexOf(tag) < 0) {
        throw fail('OperationError');
      }
      return g.__frot_subtle_gcm(open, s.bytes, copy(p.iv), copy(aad), tag / 8, data);
    },
    'AES-CBC': function (open, s, p, data) {
      if (!isBuffer(p.iv)) throw fail('InvalidAccessError');
      return g.__frot_subtle_cbc(open, s.bytes, copy(p.iv), data);
    },
    // Two NSS behaviours, both measured: it refuses a message that could bring
    // the counter back round (at most 2^length - 2 blocks, a part block counting
    // whole), and it counts in whole BYTES — a 5-bit counter carries like an
    // 8-bit one, a 127-bit one like a 128-bit one.
    'AES-CTR': function (open, s, p, data) {
      var bits = enforce(p.length, 255, 'SyntaxError');
      if (!isBuffer(p.counter)) throw fail('SyntaxError');
      if (data.length > 16 * (Math.pow(2, bits) - 2)) throw fail('OperationError');
      return g.__frot_subtle_ctr(s.bytes, copy(p.counter), Math.ceil(bits / 8) * 8, data);
    },
    // Only wrapKey/unwrapKey reach this one. Whole 64-bit words, or DataError.
    'AES-KW': function (open, s, p, data) {
      if (!data.length || data.length % 8) throw fail('DataError');
      return g.__frot_subtle_kw(open, s.bytes, data);
    },
  };
  // The key must be OF the (already normalised) algorithm. RSA-OAEP is a name
  // encrypt accepts, and no frot key is one.
  function mine(n, s) {
    if (s.algorithm.name !== n.name) throw fail('InvalidAccessError');
  }
  k.run = function (n, open, s, data) {
    mine(n, s);
    var out = CIPHER[n.name](open, s, n.p, data);
    if (!out) throw fail('OperationError');
    return out;
  };

  // --- Derivations -----------------------------------------------------------------
  // WebIDL `unsigned long` (modular, so 2^32 + 8 is 8) and whole bytes only.
  function byteCount(length) {
    var bits = length >>> 0;
    if (length === null || length === undefined || bits % 8) throw fail('OperationError');
    return bits / 8;
  }
  function hashOf(h) {
    return k.normalize(h, Object.keys(HASH)).name;
  }
  // (key slots, dictionary, bit length) -> bytes or nothing. The dictionary is
  // judged first, then the length, then the hash name, then the work itself.
  var DERIVE = {
    // NSS stops at 2048 bits of output (measured, every hash).
    PBKDF2: function (s, p, length) {
      if (!isBuffer(p.salt) || p.hash === undefined) throw fail('SyntaxError');
      var rounds = enforce(p.iterations, 4294967295, 'SyntaxError');
      var n = byteCount(length), hash = hashOf(p.hash);
      return n <= 256 && g.__frot_subtle_pbkdf2(hash, s.bytes, copy(p.salt), rounds, n);
    },
    HKDF: function (s, p, length) {
      if (!isBuffer(p.salt) || !isBuffer(p.info) || p.hash === undefined) {
        throw new TypeError('The operation could not be performed.');
      }
      var n = byteCount(length), hash = hashOf(p.hash);
      return g.__frot_subtle_hkdf(hash, s.bytes, copy(p.salt), copy(p.info), n);
    },
  };
  // ECDH and X25519 are names deriveBits accepts, and no frot key is one.
  k.derive = function (n, s, length) {
    mine(n, s);
    var out = DERIVE[n.name](s, n.p, length);
    if (!out) throw fail('OperationError');
    return out;
  };
})(globalThis);
