// `Image` and `HTMLImageElement` (bl-1404; shape read off Firefox 153.0esr,
// identity.md §3.20). `new Image()` is an <img> element that is in no tree —
// the same arena node `document.createElement('img')` makes — and
// `HTMLImageElement` is the tag-keyed `instanceof` interface for every <img>.
//
// frot fetches no image and decodes none, so an image never ARRIVES: assigning
// `src` reflects the attribute and sends nothing; neither `load` nor `error`
// fires; `complete` is false for as long as there is a `src` to wait for and
// `naturalWidth`/`naturalHeight` stay 0. That is what a real <img> reports
// between the assignment and the response, and it is true of every <img> frot
// holds, parsed or constructed. A beacon (`new Image().src = url`) is therefore
// not sent. Whether images should instead fail, or beacons go out, is a
// decision, not a gap in this file: bl-91a4. Runs after canvas.js (it shares
// `width`/`height` on Node.prototype with the canvas element).
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var proto = g.Node.prototype;

  function isImage(o) {
    return !!o && typeof o === 'object' && o.nodeType === 1 && o.tagName === 'IMG';
  }
  // WebIDL `unsigned long` reflected as a dimension: modular, and a value past
  // 2^31 - 1 falls back to 0 (measured: `new Image(-1, 2.7)` is 0 by 2).
  function dimension(v) {
    var n = v >>> 0;
    return n > 2147483647 ? 0 : n;
  }

  var HTMLImageElement = g.__frot_iface('HTMLImageElement');
  // `new Image(width, height)`: the height is only read when a width was given
  // (measured: `new Image(undefined, 5)` has neither attribute).
  var Image = function (width, height) {
    if (!new.target) throw new TypeError("Image constructor: 'new' is required");
    var img = g.document.createElement('img');
    if (width !== undefined) {
      img.width = width;
      if (height !== undefined) img.height = height;
    }
    return img;
  };
  brand(Image, 'Image');
  Object.defineProperty(Image, 'length', { value: 0, configurable: true });
  Image.prototype = HTMLImageElement.prototype;
  [HTMLImageElement, Image].forEach(function (C) {
    Object.defineProperty(C, Symbol.hasInstance, { value: isImage });
  });
  Object.defineProperty(g, 'Image', { value: Image, configurable: true, writable: true });

  // What an <img> that has not arrived reports; `undefined` on any other
  // element, like every tag-keyed member of the one Node.prototype.
  function pending(name, read) {
    Object.defineProperty(proto, name, {
      configurable: true,
      get: brand(function () {
        return isImage(this) ? read(this) : undefined;
      }, 'get ' + name),
    });
  }
  pending('complete', function (img) {
    return !img.getAttribute('src');
  });
  pending('naturalWidth', function () {
    return 0;
  });
  pending('naturalHeight', function () {
    return 0;
  });
  pending('currentSrc', function () {
    return '';
  });

  // `width`/`height` on an <img> are its attributes (there is no rendered box
  // to measure and no intrinsic size to fall back on); on anything else they
  // stay what canvas.js made them.
  ['width', 'height'].forEach(function (name) {
    var other = Object.getOwnPropertyDescriptor(proto, name);
    Object.defineProperty(proto, name, {
      configurable: true,
      get: brand(function () {
        return isImage(this) ? dimension(this.getAttribute(name)) : other.get.call(this);
      }, 'get ' + name),
      set: brand(function (v) {
        if (isImage(this)) this.setAttribute(name, String(dimension(v)));
        else other.set.call(this, v);
      }, 'set ' + name),
    });
  });
})(globalThis);
