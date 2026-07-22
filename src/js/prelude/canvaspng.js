// Deterministic PNG serialiser for the 2D-canvas masquerade (bl-05e6,
// identity.md §11). `canvas.js` turns a draw sequence into a deterministic RGBA
// pixel buffer; this module encodes that buffer into a WELL-FORMED, decodable
// `image/png` data URL, so a fingerprinter that hashes `canvas.toDataURL()` sees
// genuine PNG bytes, not a synthetic string. No compression is attempted —
// DEFLATE *stored* (uncompressed) blocks are spec-legal and keep the encoder
// small and exact. Every step is pure arithmetic over the input bytes: identical
// pixels always yield an identical PNG (the determinism the coherence bar needs).
// Runs after brand.js; exposes one NON-enumerable helper, `__frot_canvas_png`.
(function (g) {
  'use strict';

  // CRC-32 table (PNG per-chunk checksum), built once.
  var CRC = (function () {
    var t = new Uint32Array(256);
    for (var n = 0; n < 256; n++) {
      var c = n;
      for (var k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      t[n] = c >>> 0;
    }
    return t;
  })();
  function crc32(bytes) {
    var c = 0xffffffff;
    for (var i = 0; i < bytes.length; i++) c = CRC[(c ^ bytes[i]) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  }

  // Adler-32 (the zlib stream checksum).
  function adler32(bytes) {
    var a = 1;
    var b = 0;
    for (var i = 0; i < bytes.length; i++) {
      a = (a + bytes[i]) % 65521;
      b = (b + a) % 65521;
    }
    return ((b << 16) | a) >>> 0;
  }

  // Append a 4-byte big-endian integer to a plain byte array.
  function be32(arr, v) {
    arr.push((v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff);
  }

  // A PNG chunk: length(4 BE) + type(4 ASCII) + data + CRC(4 BE over type+data).
  function chunk(out, type, data) {
    be32(out, data.length);
    var td = [];
    for (var i = 0; i < type.length; i++) td.push(type.charCodeAt(i));
    for (var j = 0; j < data.length; j++) td.push(data[j]);
    for (var k = 0; k < td.length; k++) out.push(td[k]);
    be32(out, crc32(td));
  }

  // A zlib stream wrapping `raw` in DEFLATE stored (BTYPE=00) blocks, ≤65535 each.
  function zlib(raw) {
    var out = [0x78, 0x01]; // CMF/FLG: 32K window, no preset dict (0x7801 % 31 == 0).
    var i = 0;
    var n = raw.length;
    do {
      var len = Math.min(65535, n - i);
      out.push(i + len >= n ? 1 : 0); // BFINAL on the last block, BTYPE 00.
      out.push(len & 0xff, (len >>> 8) & 0xff);
      var nlen = ~len & 0xffff;
      out.push(nlen & 0xff, (nlen >>> 8) & 0xff);
      for (var j = 0; j < len; j++) out.push(raw[i + j]);
      i += len;
    } while (i < n);
    be32(out, adler32(raw));
    return out;
  }

  // Filtered scanlines: each row is a filter byte (0 = None) + its RGBA bytes.
  function scanlines(px, w, h) {
    var raw = [];
    var row = w * 4;
    for (var y = 0; y < h; y++) {
      raw.push(0);
      var base = y * row;
      for (var x = 0; x < row; x++) raw.push(px[base + x]);
    }
    return raw;
  }

  var SIG = [137, 80, 78, 71, 13, 10, 26, 10];
  function png(px, w, h) {
    var out = SIG.slice();
    var ihdr = [];
    be32(ihdr, w);
    be32(ihdr, h);
    ihdr.push(8, 6, 0, 0, 0); // 8-bit depth, colour type 6 (RGBA), no interlace.
    chunk(out, 'IHDR', ihdr);
    chunk(out, 'IDAT', zlib(scanlines(px, w, h)));
    chunk(out, 'IEND', []);
    return out;
  }

  var B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  function base64(bytes) {
    var s = '';
    var i;
    for (i = 0; i + 2 < bytes.length; i += 3) {
      var n = (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];
      s += B64[(n >>> 18) & 63] + B64[(n >>> 12) & 63] + B64[(n >>> 6) & 63] + B64[n & 63];
    }
    var rem = bytes.length - i;
    if (rem === 1) {
      var a = bytes[i] << 16;
      s += B64[(a >>> 18) & 63] + B64[(a >>> 12) & 63] + '==';
    } else if (rem === 2) {
      var b = (bytes[i] << 16) | (bytes[i + 1] << 8);
      s += B64[(b >>> 18) & 63] + B64[(b >>> 12) & 63] + B64[(b >>> 6) & 63] + '=';
    }
    return s;
  }

  function encode(px, w, h) {
    return 'data:image/png;base64,' + base64(png(px, w, h));
  }
  g.__frot_brand(encode, '__frot_canvas_png');
  Object.defineProperty(g, '__frot_canvas_png', {
    value: encode,
    configurable: true,
    writable: true,
  });
})(globalThis);
