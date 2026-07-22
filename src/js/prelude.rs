//! The bundled JS prelude (js.md §3) — the web-facing API, written in JS on top
//! of the syscall table so shim breadth grows without widening the Rust
//! interface. Files are concatenated and evaluated once, before any page
//! script, by [`super::syscall::install`].
//!
//! Coverage note (js.md §3): `cargo llvm-cov` cannot see these JS lines; the
//! prelude is exercised end-to-end by the golden fixture suite (subtask 9). The
//! Rust syscall closures it calls are covered directly in `syscall::tests`.
//!
//! This is the **core** prelude plus the framework-facing Element/Node breadth
//! (`elem.js`/`elem2.js`: the `instanceof` interface constructors, `style`,
//! `classList`, `dataset`, faithful `cloneNode`, form-control reflections, and
//! the `document`/`createDocumentFragment` breadth React/Vue/jQuery probe), the
//! geometry facade, the §7 environment breadth, the §5 event loop, and the §6
//! network layer: `Node`/`Element`/`Document`, `querySelector`,
//! `innerHTML`/`textContent`, `console`, the
//! geometry facade (`getBoundingClientRect`/`offset*`/`getComputedStyle`, js.md
//! §8), the environment shims (`env.js`: storage, cookie, `navigator`/
//! `location`/`matchMedia`, `self`/`window` aliasing, spec-legal denials), the
//! WHATWG-subset `URL`/`URLSearchParams` (`url.js`, over the `__frot_url_parse`
//! syscall), the
//! virtual-clock loop (`loop.js`: `setTimeout`/`setInterval`/
//! `requestAnimationFrame`, `addEventListener`/`dispatchEvent`, the
//! `DOMContentLoaded`/`load` lifecycle), and `fetch`/`XMLHttpRequest` over the
//! once-then-frozen subfetch cache (`net.js`, js.md §6).

/// The concatenated prelude source. Each module is an IIFE over `globalThis`, so
/// order matters only where one module extends another's globals: `elem.js`/
/// `elem2.js` (and `env.js`, `loop.js`) run after `dom.js` because they extend
/// the `document` and `Node` it defines — `elem2.js` after `elem.js` — and
/// `net.js` runs after `loop.js` because its XHR uses `g.Event`. `url.js` is
/// standalone (it extends no other module, only the `__frot_url_parse` syscall),
/// so its position is free.
pub const SOURCE: &str = concat!(
    // brand.js FIRST: the one Function#toString wrapper + branding registry every
    // later module (and the six later capability balls) registers through (§8).
    include_str!("prelude/brand.js"),
    "\n",
    include_str!("prelude/console.js"),
    "\n",
    include_str!("prelude/dom.js"),
    "\n",
    include_str!("prelude/elem.js"),
    "\n",
    include_str!("prelude/elem2.js"),
    "\n",
    include_str!("prelude/geometry.js"),
    "\n",
    // The identity surface (bl-3972), derived from __frot_env_profile: after
    // dom.js (extends document/Node), any order among themselves.
    include_str!("prelude/navigator.js"),
    "\n",
    include_str!("prelude/screen.js"),
    "\n",
    include_str!("prelude/intl.js"),
    "\n",
    include_str!("prelude/crypto.js"),
    "\n",
    include_str!("prelude/env.js"),
    "\n",
    include_str!("prelude/url.js"),
    "\n",
    include_str!("prelude/loop.js"),
    "\n",
    include_str!("prelude/net.js"),
    "\n",
    // nativebrand.js LAST: sweeps the whole existing web-API surface into the
    // brand registry so no function's toString discloses prelude source (§8).
    include_str!("prelude/nativebrand.js"),
);
