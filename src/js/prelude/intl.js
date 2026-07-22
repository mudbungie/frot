// Intl — a minimal, coherent ECMA-402 surface (identity.md §8/§9, js.md §7,
// bl-3972 / bl-ac8d). quickjs-ng ships without Intl, and its absence is a loud
// tell (every modern browser has it), so frot provides the low-entropy subset a
// page actually reads to detect locale/timezone: Intl.DateTimeFormat with a
// faithful resolvedOptions(). Locale is the persona locale and timeZone is pinned
// UTC (the §9 determinism decision — a host zone would leak entropy and vary
// output; a declared residual, §11), both from the one __frot_env_profile()
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

  var Intl = {};
  Object.defineProperty(Intl, Symbol.toStringTag, { value: 'Intl', configurable: true });
  Object.defineProperty(Intl, 'DateTimeFormat', {
    value: DateTimeFormat,
    configurable: true,
    writable: true,
    enumerable: true,
  });
  Object.defineProperty(g, 'Intl', {
    value: Intl,
    configurable: true,
    writable: true,
    enumerable: false,
  });
})(globalThis);
