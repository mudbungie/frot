// `Image` and `HTMLImageElement` (bl-1404; shape read off Firefox 153.0esr,
// identity.md §3.20). `new Image()` is an <img> element that is in no tree —
// the same arena node `document.createElement('img')` makes — and
// `HTMLImageElement` (built by dom.js) is the interface of every <img>.
//
// What an image DOES — the request its `src` starts and how it ends — is
// imageload.js's (bl-91a4); this file is the interface over it: the 27 members
// Firefox's prototype owns (tests/fixtures/js/dom-interfaces.json).
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var slots = g.__frot_slots;
  var load = g.__frot_image_hook;
  delete g.__frot_image_hook;
  var HTMLImageElement = g.HTMLImageElement;
  var proto = HTMLImageElement.prototype;

  // WebIDL `unsigned long` reflected as a dimension: modular, and a value past
  // 2^31 - 1 falls back to 0 (measured: `new Image(-1, 2.7)` is 0 by 2).
  function dimension(v) {
    var n = v >>> 0;
    return n > 2147483647 ? 0 : n;
  }
  // An attribute read as a non-negative integer: its leading digits, else 0
  // (measured: hspace="-3" is 0).
  function integer(v) {
    var n = parseInt(v, 10);
    return n > 0 && n <= 2147483647 ? n : 0;
  }

  // `new Image(width, height)`: the height is only read when a width was given
  // (measured: `new Image(undefined, 5)` has neither attribute).
  var Image = function (width, height) {
    g.__frot_new(new.target, 'Image');
    var img = g.document.createElement('img');
    if (width !== undefined) {
      img.width = width;
      if (height !== undefined) img.height = height;
    }
    return img;
  };
  brand(Image, 'Image');
  Object.defineProperty(Image, 'length', { value: 0, configurable: true });
  Image.prototype = proto;
  Object.defineProperty(g, 'Image', { value: Image, configurable: true, writable: true });

  function id(img) {
    return slots(img).id;
  }
  function accessor(get, set) {
    return { get: get, set: set };
  }
  function reflect(attr, read, write) {
    return accessor(
      function () {
        return read(this.getAttribute(attr));
      },
      function (v) {
        if (write) write(this, v);
        else this.setAttribute(attr, String(v));
      }
    );
  }
  function text(v) {
    return v || '';
  }
  // An enumerated attribute: its keyword lowercased when it is one, else the
  // invalid-value default (`bad`), else — absent — the missing-value default.
  function keyword(words, missing, bad) {
    return function (v) {
      if (v === null) return missing;
      var k = v.toLowerCase();
      return words.indexOf(k) >= 0 ? k : bad;
    };
  }
  // `width`/`height`: the attribute when there is one, else the size the image
  // arrived with (measured: a loaded `new Image()` with no attribute reports it).
  function side(name, i) {
    return accessor(
      function () {
        var v = this.getAttribute(name);
        return v === null ? load.natural(id(this), i) : dimension(v);
      },
      function (v) {
        this.setAttribute(name, String(dimension(v)));
      }
    );
  }
  function position(axis) {
    return accessor(function () {
      var r = this.getBoundingClientRect();
      return Math.round(axis === 'x' ? r.left + g.scrollX : r.top + g.scrollY);
    });
  }
  function getter(read) {
    return accessor(function () {
      return read(id(this));
    });
  }
  var src = Object.getOwnPropertyDescriptor(proto, 'src');
  var reflectUrl = g.__frot_reflect_url;
  var members = {
    alt: reflect('alt', text),
    src: src,
    srcset: reflect('srcset', text),
    crossOrigin: reflect('crossorigin', keyword(['anonymous', 'use-credentials'], null, 'anonymous'),
      function (el, v) {
        if (v === null) el.removeAttribute('crossorigin');
        else el.setAttribute('crossorigin', String(v));
      }),
    useMap: reflect('usemap', text),
    referrerPolicy: reflect('referrerpolicy', keyword(['no-referrer', 'no-referrer-when-downgrade',
      'same-origin', 'origin', 'strict-origin', 'origin-when-cross-origin',
      'strict-origin-when-cross-origin', 'unsafe-url'], '', '')),
    isMap: reflect('ismap', function (v) {
      return v !== null;
    }, function (el, v) {
      if (v) el.setAttribute('ismap', '');
      else el.removeAttribute('ismap');
    }),
    width: side('width', 0),
    height: side('height', 1),
    decoding: reflect('decoding', keyword(['sync', 'async', 'auto'], 'auto', 'auto')),
    loading: reflect('loading', keyword(['lazy', 'eager'], 'eager', 'eager')),
    fetchPriority: reflect('fetchpriority', keyword(['high', 'low', 'auto'], 'auto', 'auto')),
    naturalWidth: getter(function (n) {
      return load.natural(n, 0);
    }),
    naturalHeight: getter(function (n) {
      return load.natural(n, 1);
    }),
    complete: getter(load.complete),
    name: reflect('name', text),
    align: reflect('align', text),
    hspace: reflect('hspace', integer),
    vspace: reflect('vspace', integer),
    longDesc: reflectUrl('longdesc'),
    border: reflect('border', text),
    sizes: reflect('sizes', text),
    currentSrc: getter(load.currentSrc),
    lowsrc: reflectUrl('lowsrc'),
    x: position('x'),
    y: position('y'),
  };
  // Their order is Firefox's, set once with every interface's (nativebrand.js).
  g.__frot_ifaceops(proto, {
    decode: function decode() {
      return load.decode(id(this));
    },
  });
  Object.keys(members).forEach(function (k) {
    var d = members[k];
    if (d.get) brand(d.get, 'get ' + k);
    if (d.set) brand(d.set, 'set ' + k);
    d.enumerable = true;
    d.configurable = true;
    Object.defineProperty(proto, k, d);
  });

  // HTMLSourceElement's selection inputs (bl-0e4a): what a framework sets on
  // a `<picture>`'s `<source>` — reflected, so imageload.js's attribute seam
  // sees them (measured: `width = '7x'` and `height = -3` both write "0").
  var source = {
    srcset: reflect('srcset', text),
    sizes: reflect('sizes', text),
    media: reflect('media', text),
    width: reflect('width', integer, function (el, v) {
      el.setAttribute('width', String(dimension(v)));
    }),
    height: reflect('height', integer, function (el, v) {
      el.setAttribute('height', String(dimension(v)));
    }),
  };
  Object.keys(source).forEach(function (k) {
    var d = source[k];
    brand(d.get, 'get ' + k);
    brand(d.set, 'set ' + k);
    d.configurable = true;
    Object.defineProperty(g.HTMLSourceElement.prototype, k, d);
  });
})(globalThis);
