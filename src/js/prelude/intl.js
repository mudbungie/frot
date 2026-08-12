// Intl — a minimal, coherent ECMA-402 surface (identity.md §8/§9, js.md §7,
// bl-3972 / bl-ac8d). quickjs-ng ships without Intl, and its absence is a loud
// tell (every modern browser has it), so frot provides the low-entropy subset a
// page actually reads to detect locale/timezone: Intl.DateTimeFormat with a
// faithful resolvedOptions(). Locale is the persona locale and timeZone is pinned
// UTC (the §9 determinism decision — a host zone would leak entropy and vary
// output; a declared identity.md §11 residual), both from the one __frot_env_profile()
// channel, so JS locale, the Accept-Language header, and navigator.language all
// agree. Full NumberFormat/Collator/relative-time formatting stays a residual.
(function (g) {
  'use strict';
  var P = JSON.parse(g.__frot_env_profile());

  function hidden(o, k, v) {
    Object.defineProperty(o, k, { value: v, configurable: true });
  }

  function DateTimeFormat(locales, options) {
    if (!(this instanceof DateTimeFormat)) return new DateTimeFormat(locales, options);
    var loc = Array.isArray(locales) ? locales[0] : locales;
    hidden(this, '_locale', typeof loc === 'string' && loc ? loc : P.locale);
    hidden(this, '_timeZone', options && options.timeZone ? options.timeZone : P.timeZone);
  }
  DateTimeFormat.prototype.resolvedOptions = g.__frot_brand(function resolvedOptions() {
    return {
      locale: this._locale,
      calendar: 'gregory',
      numberingSystem: 'latn',
      timeZone: this._timeZone,
      year: 'numeric',
      month: 'numeric',
      day: 'numeric',
    };
  });
  // A best-effort en-US m/d/yyyy in the resolved (UTC) zone — enough that a page
  // formatting a date does not throw; the identity surface is resolvedOptions().
  DateTimeFormat.prototype.format = g.__frot_brand(function format(date) {
    var d = date === undefined ? new Date() : new Date(date);
    return d.getUTCMonth() + 1 + '/' + d.getUTCDate() + '/' + d.getUTCFullYear();
  });
  g.__frot_brand(DateTimeFormat, 'DateTimeFormat');
  // `Object.prototype.toString.call(new Intl.DateTimeFormat())` is
  // `[object Intl.DateTimeFormat]`, not `[object Object]` — the tag is on the
  // PROTOTYPE and it carries the `Intl.` qualifier. Descriptor and value both
  // READ off Firefox 153.0esr (`bl-1ab7`, identity.md §3.12).
  Object.defineProperty(DateTimeFormat.prototype, Symbol.toStringTag, {
    value: 'Intl.DateTimeFormat', writable: false, enumerable: false, configurable: true,
  });

  var Intl = {};
  Object.defineProperty(Intl, Symbol.toStringTag, { value: 'Intl', configurable: true });
  // Non-enumerable: measured, `Object.keys(Intl)` is `[]` on a real Firefox even
  // though it carries twelve constructors. An enumerable one would make frot the
  // only browser whose `Object.keys(Intl)` is non-empty.
  Object.defineProperty(Intl, 'DateTimeFormat', {
    value: DateTimeFormat,
    configurable: true,
    writable: true,
    enumerable: false,
  });
  Object.defineProperty(g, 'Intl', {
    value: Intl,
    configurable: true,
    writable: true,
    enumerable: false,
  });
})(globalThis);
