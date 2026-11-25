// WebCrypto's RSA half (bl-2763): RSASSA-PKCS1-v1_5, RSA-PSS and RSA-OAEP —
// `spki`/`pkcs8` DER and RSA JWKs, generateKey, sign/verify, encrypt/decrypt and
// wrapKey/unwrapKey. Fourth of five modules (after subtlepk.js, before
// subtle.js); every rule, shape and error below was READ off Firefox 153.0esr
// (identity.md §3.19 rig; src/js/tests/subtle/firefox153rsa.txt). The arithmetic
// is aws-lc-rs's through the __frot_subtle_rsa_* syscalls; the DER is written and
// read here. What the provider cannot compute rejects NotSupportedError where
// Firefox succeeds — the declared residual (identity.md §3.19): a key outside
// 2048–8192 bits, signing with SHA-1, a PSS salt other than the digest's length,
// generating any size but 2048/3072/4096/8192 or any exponent but 65537.
// Wraps the toolkit's pk* entry points and `run`, handing anything not RSA on.
(function (g) {
  'use strict';
  var k = g.__frot_subtle;
  if (!k) return; // insecure context
  var fail = k.fail, slots = g.__frot_slots, tlv = k.tlv, items = k.items, copy = k.copy;
  var HLEN = { 'SHA-1': 20, 'SHA-256': 32, 'SHA-384': 48, 'SHA-512': 64 };
  var MEMBERS = ['n', 'e', 'd', 'p', 'q', 'dp', 'dq', 'qi'];
  var PRIVATE = MEMBERS.slice(2);
  // rsaEncryption, with its NULL parameters.
  var ALG_ID = tlv(0x30, tlv(6, k.hexb('2a864886f70d010101')).concat([5, 0]));

  // --- DER ------------------------------------------------------------------------
  // An unsigned integer: its bytes without leading zeros.
  function uint(bytes) {
    var i = 0;
    while (i < bytes.length && bytes[i] === 0) i++;
    return bytes.subarray(i);
  }
  function int(bytes) {
    var b = Array.from(bytes);
    return tlv(2, !b.length || b[0] & 0x80 ? [0].concat(b) : b);
  }
  function spki(m) {
    var key = tlv(0x30, int(m.n).concat(int(m.e)));
    return Uint8Array.from(tlv(0x30, ALG_ID.concat(tlv(3, [0].concat(key)))));
  }
  function pkcs8(m) {
    var body = [2, 1, 0];
    MEMBERS.forEach(function (x) {
      body = body.concat(int(m[x]));
    });
    var key = tlv(0x30, body);
    return Uint8Array.from(tlv(0x30, [2, 1, 0].concat(ALG_ID, tlv(4, key))));
  }
  function algorithmId(body) {
    var id = items(body, [6, 5], 1);
    if (!k.same(id[0][1], k.hexb('2a864886f70d010101'))) throw fail('DataError');
  }
  function unspki(der) {
    var it = items(items(der, [0x30], 1)[0][1], [0x30, 3], 2);
    algorithmId(it[0][1]);
    if (it[1][1][0] !== 0) throw fail('DataError');
    var key = items(items(it[1][1].subarray(1), [0x30], 1)[0][1], [2, 2], 2);
    return { n: uint(key[0][1]), e: uint(key[1][1]) };
  }
  function unpkcs8(der) {
    var it = items(items(der, [0x30], 1)[0][1], [2, 0x30, 4], 3);
    algorithmId(it[1][1]);
    var key = items(items(it[2][1], [0x30], 1)[0][1], [2, 2, 2, 2, 2, 2, 2, 2, 2], 9);
    var m = {};
    MEMBERS.forEach(function (x, i) {
      m[x] = uint(key[i + 1][1]);
    });
    return m;
  }
  function bitLength(n) {
    var bits = 0;
    for (var top = n[0]; top; top >>= 1) bits++;
    return (n.length - 1) * 8 + bits;
  }
  // The DER the provider takes, or why it does not: a size outside 2048–8192 is
  // the residual; so is a public key it will not take (an even modulus, an
  // exponent of 1 or 2), which Firefox takes as it comes. A private key it will
  // not take is not a key.
  function accepted(m, priv) {
    var der = priv ? pkcs8(m) : spki(m);
    var verdict = g.__frot_subtle_rsa_check(priv, der);
    if (verdict < 0 || (!verdict && !priv)) throw fail('NotSupportedError');
    if (!verdict) throw fail('DataError');
    return der;
  }

  // --- Keys -------------------------------------------------------------------------
  // The hash a key carries: any recognised algorithm name, not only a digest.
  function hashOf(p, missing, unknown) {
    if (p.hash === undefined) throw fail(missing);
    try {
      return k.normalize(p.hash, Object.keys(k.HASH).concat(k.KEYED)).name;
    } catch (e) {
      throw fail(unknown);
    }
  }
  function family(name, priv, pub, jwkAlg) {
    return {
      pk: true, rsa: true, name: name, priv: priv, pub: pub, jwkAlg: jwkAlg,
      usages: priv.concat(pub), params: function () {},
      generated: function () {
        throw fail('NotSupportedError');
      },
    };
  }
  function suffixed(prefix) {
    return function (hash) {
      return HLEN[hash] && prefix + hash.slice(4).replace('-', '');
    };
  }
  var RSA = {
    'RSASSA-PKCS1-v1_5': family('RSASSA-PKCS1-v1_5', ['sign'], ['verify'], suffixed('RS')),
    'RSA-PSS': family('RSA-PSS', ['sign'], ['verify'], suffixed('PS')),
    'RSA-OAEP': family('RSA-OAEP', ['decrypt', 'unwrapKey'], ['encrypt', 'wrapKey'],
      function (hash) {
        return HLEN[hash] && (hash === 'SHA-1' ? 'RSA-OAEP' : 'RSA-OAEP-' + hash.slice(4));
      }),
  };
  Object.assign(k.ALG, RSA);

  function makeRsa(impl, hash, m, exponent, priv, extractable, usages, der) {
    var key = Object.create(g.CryptoKey.prototype);
    Object.assign(slots(key), {
      type: priv ? 'private' : 'public', extractable: !!extractable, usages: usages,
      algorithm: { name: impl.name, hash: { name: hash }, modulusLength: bitLength(m.n),
        publicExponent: exponent },
      pk: { rsa: m, der: der },
    });
    return key;
  }
  function fromJwk(jwk, priv) {
    if (jwk.kty !== 'RSA') throw fail('DataError');
    var m = {};
    (priv ? MEMBERS : MEMBERS.slice(0, 2)).forEach(function (x) {
      m[x] = uint(k.unb64url(jwk[x]));
    });
    return m;
  }

  var pkImport = k.pkImport;
  k.pkImport = function (format, data, n, extractable, list) {
    if (!n.impl.rsa) return pkImport(format, data, n, extractable, list);
    var impl = n.impl, hash = hashOf(n.p, 'DataError', 'DataError');
    if (format === 'raw') throw fail('SyntaxError');
    var jwk = format === 'jwk' ? data : null;
    var priv = jwk ? jwk.d !== undefined : format === 'pkcs8';
    if (!jwk && !k.isBuffer(data)) throw fail('DataError');
    var m = jwk ? fromJwk(jwk, priv) : priv ? unpkcs8(copy(data)) : unspki(copy(data));
    var der = accepted(m, priv);
    if (!priv && !list.length) list = [];
    else list = k.usageList(list, priv ? impl.priv : impl.pub);
    if (jwk) {
      var ops = jwk.key_ops, alg = impl.jwkAlg(hash);
      if ((!priv && PRIVATE.some(function (x) {
        return jwk[x] !== undefined;
      })) || (jwk.ext === false && extractable)
        || (jwk.alg !== undefined && jwk.alg !== alg)
        || (ops !== undefined && !(Array.isArray(ops) && list.every(function (u) {
          return ops.indexOf(u) >= 0;
        })))) throw fail('DataError');
    }
    return makeRsa(impl, hash, m, Uint8Array.from(m.e), priv, extractable, list, der);
  };

  var pkExport = k.pkExport;
  k.pkExport = function (format, s) {
    var m = s.pk.rsa, priv = s.type === 'private';
    if (!m) return pkExport(format, s);
    if (format !== 'jwk') {
      if (format !== (priv ? 'pkcs8' : 'spki')) throw fail('NotSupportedError');
      return copy(s.pk.der);
    }
    var alg = k.ALG[s.algorithm.name].jwkAlg(s.algorithm.hash.name), b64 = k.b64url;
    return Object.assign(alg ? { alg: alg } : {},
      priv ? { d: b64(m.d), dp: b64(m.dp), dq: b64(m.dq) } : {},
      { e: b64(m.e), ext: true, key_ops: s.usages.slice(), kty: 'RSA', n: b64(m.n) },
      priv ? { p: b64(m.p), q: b64(m.q), qi: b64(m.qi) } : {});
  };

  // The dictionary, then the usages (the private half must take one), then the
  // values: a size or exponent Firefox refuses is OperationError, one it takes
  // and the provider cannot make is the residual.
  var pkGenerate = k.pkGenerate;
  k.pkGenerate = function (n, extractable, list) {
    var impl = n.impl, p = n.p;
    if (!impl.rsa) return pkGenerate(n, extractable, list);
    if (p.hash === undefined || p.modulusLength === undefined
      || !(p.publicExponent instanceof Uint8Array)) throw fail('SyntaxError');
    var hash = hashOf(p, 'SyntaxError', 'NotSupportedError');
    var all = k.usageList(list, impl.usages);
    var mine = all.filter(function (u) {
      return impl.priv.indexOf(u) >= 0;
    });
    if (!mine.length) throw fail('SyntaxError');
    var bits = Math.trunc(Number(p.modulusLength)), e = Array.from(p.publicExponent);
    if (e.length > 8) throw fail('InvalidAccessError');
    var value = e.reduce(function (a, b) {
      return a * 256 + b;
    }, 0);
    if (bits % 16 || !(bits >= 256 && bits <= 16384) || value < 3 || value % 2 === 0
      || value >= 4294967296) throw fail('OperationError');
    var made = g.__frot_subtle_rsa_generate(value === 65537 ? bits : 0);
    if (!made) throw fail('NotSupportedError');
    var m = unpkcs8(Uint8Array.from(made[0]));
    return {
      privateKey: makeRsa(impl, hash, m, copy(p.publicExponent), true, extractable, mine,
        pkcs8(m)),
      publicKey: makeRsa(impl, hash, m, copy(p.publicExponent), false, true,
        all.filter(function (u) {
          return impl.pub.indexOf(u) >= 0;
        }), spki(m)),
    };
  };

  // The key's own hash. PSS takes a salt length: absent or negative is
  // SyntaxError, longer than the modulus leaves room for OperationError.
  function scheme(n, s) {
    var hash = s.algorithm.hash.name, pss = n.name === 'RSA-PSS';
    if (pss) {
      var salt = n.p.saltLength === undefined ? -1 : Number(n.p.saltLength);
      if (!(salt >= 0)) throw fail('SyntaxError');
      if (salt > s.pk.rsa.n.length - (HLEN[hash] || 0) - 2) throw fail('OperationError');
      if (salt !== HLEN[hash]) throw fail('NotSupportedError');
    }
    if (!HLEN[hash] || (hash === 'SHA-1' && (pss || s.type === 'private'))) {
      throw fail('NotSupportedError');
    }
    return [pss, hash];
  }
  var pkSign = k.pkSign, pkVerify = k.pkVerify;
  k.pkSign = function (n, s, data) {
    if (!s.pk.rsa) return pkSign(n, s, data);
    var how = scheme(n, s);
    return g.__frot_subtle_rsa_sign(how[0], how[1], s.pk.der, data);
  };
  k.pkVerify = function (n, s, sig, data) {
    if (!s.pk.rsa) return pkVerify(n, s, sig, data);
    var how = scheme(n, s);
    return g.__frot_subtle_rsa_verify(how[0], how[1], s.pk.der, sig, data);
  };

  // RSA-OAEP: a `label` must be a buffer; data too long, or a ciphertext that
  // does not decrypt, is OperationError.
  var run = k.run;
  k.run = function (n, open, s, data) {
    if (n.name !== 'RSA-OAEP') return run(n, open, s, data);
    if (s.algorithm.name !== n.name) throw fail('InvalidAccessError');
    var label = n.p.label === undefined ? new Uint8Array(0) : n.p.label;
    if (!k.isBuffer(label)) throw fail('SyntaxError');
    var hash = s.algorithm.hash.name;
    if (!HLEN[hash]) throw fail('NotSupportedError');
    var out = g.__frot_subtle_rsa_oaep(open, hash, s.pk.der, copy(label), data);
    if (!out) throw fail('OperationError');
    return out;
  };
})(globalThis);
