// WebCrypto's elliptic-curve half (bl-0cae): ECDSA and ECDH over P-256/384/521,
// Ed25519 and X25519 — keys that come in pairs, `spki`/`pkcs8` DER, EC and OKP
// JWKs, signatures and agreements. Third of four modules (after subtlekey.js
// and subtlealg.js, before subtle.js); every rule, shape and error below was
// READ off Firefox 153.0esr (identity.md §3.19 rig; the 331-call capture
// src/js/tests/subtle/firefox153pk.txt). The arithmetic is the provider's,
// through the __frot_subtle_pk_* / ecdsa / ed25519 / agree syscalls; the DER
// is written and read here. RSA* is not here: it stays NotSupportedError.
// Adds the four algorithms to `ALG` and `pkImport`, `pkExport`, `pkGenerate`,
// `pkSign`, `pkVerify` to the toolkit.
(function (g) {
  'use strict';
  var k = g.__frot_subtle;
  if (!k) return; // insecure context
  var fail = k.fail, slots = g.__frot_slots;
  var hexb = k.hexb, tlv = k.tlv, items = k.items, same = k.same;

  // A NIST curve: coordinate/scalar bytes, its OID and ECDSA's JWK `alg`.
  var CURVES = {
    'P-256': [32, '2a8648ce3d030107', 'ES256'],
    'P-384': [48, '2b81040022', 'ES384'],
    'P-521': [66, '2b81040023', 'ES512'],
  };
  var OID = { EC: '2a8648ce3d0201', Ed25519: '2b6570', X25519: '2b656e' };

  // --- DER ------------------------------------------------------------------------
  // The algorithm identifier a key of `curve` is written under; and the curve
  // one names, read back. An EC key's curve is the one its DER names, whatever
  // the call asked for; the two OKP identifiers are told apart by nothing but
  // the call (both measured).
  function algId(curve) {
    var c = CURVES[curve];
    return tlv(0x30, c ? tlv(6, hexb(OID.EC)).concat(tlv(6, hexb(c[1]))) : tlv(6, hexb(OID[curve])));
  }
  function curveOf(body, asked) {
    var id = items(body, [6, 6], 1).map(function (t) {
      return Array.from(t[1], function (b) {
        return (b + 256).toString(16).slice(1);
      }).join('');
    });
    var found = CURVES[asked] ? Object.keys(CURVES).filter(function (c) {
      return id[0] === OID.EC && CURVES[c][1] === id[1];
    })[0] : (id[0] === OID.Ed25519 || id[0] === OID.X25519) && id.length === 1 && asked;
    if (!found) throw fail('DataError');
    return found;
  }
  function bits(point) {
    return tlv(3, [0].concat(Array.from(point)));
  }
  function unbits(body) {
    if (body[0] !== 0) throw fail('DataError');
    return body.subarray(1);
  }
  function spki(curve, pub) {
    return Uint8Array.from(tlv(0x30, algId(curve).concat(bits(pub))));
  }
  function unspki(curve, der) {
    var it = items(items(der, [0x30], 1)[0][1], [0x30, 3], 2);
    return { curve: curveOf(it[0][1], curve), d: null, pub: unbits(it[1][1]) };
  }
  // PKCS #8: an EC key carries its point when the key has one; an OKP key
  // never does (version 0, no attributes).
  function pkcs8(curve, d, pub) {
    var inner = CURVES[curve]
      ? tlv(0x30, [2, 1, 1].concat(tlv(4, d), pub ? tlv(0xa1, bits(pub)) : []))
      : tlv(4, d);
    return Uint8Array.from(tlv(0x30, [2, 1, 0].concat(algId(curve), tlv(4, inner))));
  }
  // An OKP key's public half is computed from its seed; an EC key has the
  // point its DER carries, or none.
  function unpkcs8(curve, der) {
    var it = items(items(der, [0x30], 1)[0][1], [2, 0x30, 4], 3);
    var c = curveOf(it[1][1], curve);
    if (!CURVES[c]) {
      var seed = items(it[2][1], [4], 1)[0][1];
      var pub = g.__frot_subtle_pk_public(c, seed);
      if (!pub) throw fail('DataError');
      return { curve: c, d: seed, pub: pub };
    }
    var ec = items(items(it[2][1], [0x30], 1)[0][1], [2, 4, 0xa0, 0xa1], 2);
    function tagged(tag) {
      return ec.filter(function (t) {
        return t[0] === tag;
      })[0];
    }
    // Parameters inside the key, if any, must name the same curve.
    var params = tagged(0xa0), pub = tagged(0xa1);
    if (params && !same(params[1], tlv(6, hexb(CURVES[c][1])))) throw fail('DataError');
    return { curve: c, d: ec[1][1], pub: pub ? unbits(items(pub[1], [3], 1)[0][1]) : null };
  }

  // --- Keys -------------------------------------------------------------------------
  // `curve` reads the dictionary; `priv`/`pub` are the usages each half takes.
  function named(p) {
    if (p.namedCurve === undefined) throw fail('SyntaxError');
    var want = String(p.namedCurve).toUpperCase();
    var c = Object.keys(CURVES).filter(function (n) {
      return n === want;
    })[0];
    if (!c) throw fail('NotSupportedError');
    return c;
  }
  function family(name, kty, priv, pub, jwkAlgs) {
    var okp = kty === 'OKP';
    return {
      pk: true, kty: kty, priv: priv, pub: pub, jwkAlgs: jwkAlgs, params: function () {},
      curve: okp ? function () {
        return name;
      } : named,
      algorithm: function (curve) {
        return okp ? { name: name } : { name: name, namedCurve: curve };
      },
      generated: function () {
        throw fail('NotSupportedError');
      },
      usages: priv.concat(pub),
    };
  }
  var DERIVES = ['deriveKey', 'deriveBits'];
  // The JWK `alg`s a key may carry; the first is the one it is exported with.
  function none() {
    return [];
  }
  var PK = {
    ECDSA: family('ECDSA', 'EC', ['sign'], ['verify'], function (c) {
      return [CURVES[c][2]];
    }),
    ECDH: family('ECDH', 'EC', DERIVES, [], none),
    Ed25519: family('Ed25519', 'OKP', ['sign'], ['verify'], function () {
      return ['Ed25519', 'EdDSA'];
    }),
    X25519: family('X25519', 'OKP', DERIVES, [], none),
  };
  Object.assign(k.ALG, PK);

  function makePk(impl, curve, type, extractable, usages, d, pub) {
    var key = Object.create(g.CryptoKey.prototype);
    Object.assign(slots(key), {
      type: type, extractable: !!extractable, algorithm: impl.algorithm(curve), usages: usages,
      pk: { curve: curve, d: d, pub: pub },
    });
    return key;
  }
  // A public point must be one of the curve's, kept uncompressed. An EC scalar is taken as given
  // — any length, matching the point or not (measured) — where an Ed25519 JWK's
  // `x` must be its seed's public key. X25519's need not be (measured).
  function checked(m) {
    if (m.pub && !(m.pub = g.__frot_subtle_pk_point(m.curve, m.pub))) throw fail('DataError');
    return m;
  }
  function jwkMaterial(impl, curve, jwk) {
    var c = CURVES[curve], n = c ? c[0] : 32;
    if (jwk.kty !== impl.kty || (c ? jwk.crv !== curve : !OID[jwk.crv] || jwk.crv === 'EC')) {
      throw fail('DataError');
    }
    var x = k.unb64url(jwk.x), y = c ? k.unb64url(jwk.y) : x;
    var d = jwk.d === undefined ? null : k.unb64url(jwk.d);
    if (x.length !== n || y.length !== n || (d && (c ? !d.length : d.length !== 32))) {
      throw fail('DataError');
    }
    var pub = c ? Uint8Array.from([4].concat(Array.from(x), Array.from(y))) : x;
    if (d && curve === 'Ed25519' && !same(g.__frot_subtle_pk_public(curve, d), pub)) {
      throw fail('DataError');
    }
    return checked({ curve: curve, d: d, pub: pub });
  }
  function wanted(list, allowed, empty) {
    if (!empty && !list.length) throw fail('SyntaxError');
    return list.length ? k.usageList(list, allowed) : [];
  }

  k.pkImport = function (format, data, n, extractable, list) {
    var impl = n.impl, curve = impl.curve(n.p);
    var jwk = format === 'jwk' ? data : null;
    var priv = jwk ? jwk.d !== undefined : format === 'pkcs8';
    // The key data is judged before the usages (measured), a JWK's own
    // members after them.
    if (!jwk && !k.isBuffer(data)) throw fail('DataError');
    var bytes = jwk || k.copy(data);
    var m = jwk ? jwkMaterial(impl, curve, jwk) : checked(format === 'pkcs8' ? unpkcs8(curve, bytes)
      : format === 'spki' ? unspki(curve, bytes) : { curve: curve, d: null, pub: bytes });
    var use = wanted(list, priv ? impl.priv : impl.pub, !priv);
    if (jwk) {
      var ops = jwk.key_ops, algs = impl.jwkAlgs(curve);
      if ((jwk.ext === false && extractable)
        || (jwk.alg !== undefined && algs.length && algs.indexOf(jwk.alg) < 0)
        || (ops !== undefined && !(Array.isArray(ops) && use.every(function (u) {
          return ops.indexOf(u) >= 0;
        })))) throw fail('DataError');
    }
    return makePk(impl, m.curve, priv ? 'private' : 'public', extractable, use, m.d, m.pub);
  };

  k.pkExport = function (format, s) {
    var pk = s.pk, priv = s.type === 'private', curve = pk.curve, c = CURVES[curve];
    if (format === 'pkcs8') {
      if (!priv) throw fail('NotSupportedError');
      return pkcs8(curve, pk.d, pk.pub);
    }
    if (format !== 'jwk') {
      if (priv) throw fail('NotSupportedError');
      return format === 'raw' ? k.copy(pk.pub) : spki(curve, pk.pub);
    }
    if (!pk.pub) throw fail('OperationError');
    var alg = k.ALG[s.algorithm.name].jwkAlgs(curve)[0];
    var x = c ? pk.pub.subarray(1, 1 + c[0]) : pk.pub;
    return Object.assign(alg ? { alg: alg } : {}, { crv: curve },
      priv ? { d: k.b64url(pk.d) } : {},
      { ext: true, key_ops: s.usages.slice(), kty: c ? 'EC' : 'OKP', x: k.b64url(x) },
      c ? { y: k.b64url(pk.pub.subarray(1 + c[0])) } : {});
  };

  // The private half takes the private usages and must take one; the public
  // half takes the rest, and is always extractable.
  k.pkGenerate = function (n, extractable, list) {
    var impl = n.impl, curve = impl.curve(n.p);
    var all = k.usageList(list, impl.usages);
    var mine = all.filter(function (u) {
      return impl.priv.indexOf(u) >= 0;
    });
    if (!mine.length) throw fail('SyntaxError');
    var made = g.__frot_subtle_pk_generate(curve).map(function (a) {
      return Uint8Array.from(a);
    });
    return {
      privateKey: makePk(impl, curve, 'private', extractable, mine, made[0], made[1]),
      publicKey: makePk(impl, curve, 'public', true, all.filter(function (u) {
        return impl.pub.indexOf(u) >= 0;
      }), null, made[1]),
    };
  };

  // ECDSA's hash is a required member: absent is SyntaxError, unknown
  // NotSupportedError. Ed25519 takes none.
  function ecdsaHash(p) {
    if (p.hash === undefined) throw fail('SyntaxError');
    return k.normalize(p.hash, Object.keys(k.HASH)).name;
  }
  // A scalar the provider cannot sign with (zero, or out of range) is
  // OperationError at the signature (measured).
  k.pkSign = function (n, s, data) {
    var sig = n.name === 'Ed25519' ? g.__frot_subtle_ed25519_sign(s.pk.d, data)
      : g.__frot_subtle_ecdsa_sign(s.pk.curve, ecdsaHash(n.p), s.pk.d, data);
    if (!sig) throw fail('OperationError');
    return sig;
  };
  k.pkVerify = function (n, s, sig, data) {
    if (n.name === 'Ed25519') return g.__frot_subtle_ed25519_verify(s.pk.pub, sig, data);
    return g.__frot_subtle_ecdsa_verify(s.pk.curve, ecdsaHash(n.p), s.pk.pub, sig, data);
  };

  // ECDH and X25519: the peer's public key of the same algorithm and curve;
  // `length` null takes the whole secret, a part of it is cut to whole bytes
  // with the bits past it cleared, and more than it is OperationError.
  function agree(s, p, length) {
    var peer = slots(p.public);
    if (!peer.type) throw new TypeError('The operation could not be performed.');
    if (peer.type !== 'public' || peer.algorithm.name !== s.algorithm.name
      || peer.pk.curve !== s.pk.curve) throw fail('InvalidAccessError');
    var secret = g.__frot_subtle_agree(s.pk.curve, s.pk.d, peer.pk.pub);
    if (!secret) throw fail('OperationError');
    var want = length === null || length === undefined ? secret.length * 8 : length >>> 0;
    if (want > secret.length * 8) throw fail('OperationError');
    var out = secret.slice(0, Math.ceil(want / 8));
    if (want % 8) out[out.length - 1] &= 0xff << (8 - want % 8);
    return out;
  }
  k.DERIVE.ECDH = agree;
  k.DERIVE.X25519 = agree;
})(globalThis);
