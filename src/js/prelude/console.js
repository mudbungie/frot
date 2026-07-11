// console — routes every level through the single __frot_console syscall.
// Arguments are rendered to one space-joined string (strings verbatim, other
// values JSON-stringified where possible) so the host captures a flat line.
(function (g) {
  'use strict';
  function format(v) {
    if (typeof v === 'string') return v;
    if (v === undefined) return 'undefined';
    if (v === null) return 'null';
    try {
      return JSON.stringify(v);
    } catch (e) {
      return String(v);
    }
  }
  function make(level) {
    return function () {
      var parts = [];
      for (var i = 0; i < arguments.length; i++) parts.push(format(arguments[i]));
      g.__frot_console(level, parts.join(' '));
    };
  }
  g.console = {
    log: make('log'),
    info: make('info'),
    warn: make('warn'),
    error: make('error'),
    debug: make('debug'),
  };
})(globalThis);
