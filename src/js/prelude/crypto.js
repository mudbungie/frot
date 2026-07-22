// crypto — real OS randomness with browser argument/error behaviour (identity.md
// §8/§11, js.md §7, bl-3972 / bl-cf3a). getRandomValues/randomUUID are cheap
// capabilities frot provides *genuinely*, not as a deterministic fake: the bytes
// come from the __frot_random_bytes syscall (/dev/urandom), so separate calls
// share no state and cannot be pinned (OQ-2 resolved against seeding). The
// Web Crypto argument/quota/error contract is enforced here in JS. crypto.subtle
// (the full SubtleCrypto surface) stays a declared residual (§11). Runs after
// brand.js (needs __frot_iface).
(function (g) {
  'use strict';

  // A DOMException-shaped error: browsers reject with a named DOMException, which
  // quickjs lacks, so a plain error carrying the spec `name` is the closest tell.
  function domError(name, message) {
    var e = new Error(message);
    e.name = name;
    return e;
  }

  function getRandomValues(array) {
    if (
      !ArrayBuffer.isView(array) ||
      array instanceof DataView ||
      array instanceof Float32Array ||
      array instanceof Float64Array
    ) {
      throw domError('TypeMismatchError', 'The provided ArrayBufferView is not an integer-typed view');
    }
    if (array.byteLength > 65536) {
      throw domError(
        'QuotaExceededError',
        "The ArrayBufferView's byte length (" + array.byteLength + ') exceeds the number of bytes of entropy available (65536)'
      );
    }
    var bytes = g.__frot_random_bytes(array.byteLength);
    var u8 = new Uint8Array(array.buffer, array.byteOffset, array.byteLength);
    for (var i = 0; i < bytes.length; i++) u8[i] = bytes[i];
    return array;
  }

  function randomUUID() {
    var b = g.__frot_random_bytes(16);
    b[6] = (b[6] & 0x0f) | 0x40; // version 4
    b[8] = (b[8] & 0x3f) | 0x80; // variant 10xx
    var s = '';
    for (var i = 0; i < 16; i++) {
      s += (b[i] + 0x100).toString(16).slice(1);
      if (i === 3 || i === 5 || i === 7 || i === 9) s += '-';
    }
    return s;
  }

  var Crypto = g.__frot_iface('Crypto', {});
  Crypto.prototype.getRandomValues = g.__frot_brand(getRandomValues, 'getRandomValues');
  Crypto.prototype.randomUUID = g.__frot_brand(randomUUID, 'randomUUID');
  var crypto = Object.create(Crypto.prototype);
  Object.defineProperty(g, 'crypto', {
    value: crypto,
    configurable: true,
    enumerable: true,
    writable: true,
  });
})(globalThis);
