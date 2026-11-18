// requestIdleCallback / cancelIdleCallback / IdleDeadline (bl-b584) — read by
// AWS WAF's challenge (challenge.md §11.4). Built on loop.js's timer queue
// through its `__frot_idle_hook` seam, which this module removes.
//
// An idle period is "nothing else is due". The order was read off Firefox
// 153.0esr (identity.md §3.20): a timeout queued AFTER the idle request still
// runs before it; idle callbacks run in request order; one requested during an
// idle callback waits for a later period. So are the shape, the handle numbering
// (its own sequence from 1), the callback's `this`, and every error message.
//
// Declared (identity.md §11): the virtual page is idle as soon as its due
// timers have run, so the period always arrives — `didTimeout` is always false
// and the `timeout` option has nothing to do. Runs after loop.js.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;
  var hook = g.__frot_idle_hook;
  delete g.__frot_idle_hook;

  var IdleDeadline = g.__frot_iface('IdleDeadline');
  g.__frot_ifaceops(IdleDeadline.prototype, {
    // Time left in this idle period, counted down on the one clock; 0 once the
    // callback has returned (measured).
    timeRemaining: function timeRemaining() {
      var s = slots(this);
      if (s.until === undefined) {
        throw new TypeError("'timeRemaining' called on an object that does not implement "
          + 'interface IdleDeadline.');
      }
      return s.open ? Math.max(0, s.until - hook.observe()) : 0;
    },
  });
  g.__frot_ifaceattrs(IdleDeadline.prototype, ['didTimeout']);

  var nextHandle = 1;
  g.requestIdleCallback = function requestIdleCallback(callback) {
    var at = 'Window.requestIdleCallback: ', options = arguments[1];
    if (!arguments.length) {
      throw new TypeError(at + 'At least 1 argument required, but only 0 passed');
    }
    if (typeof callback !== 'function') {
      var object = callback !== null && typeof callback === 'object';
      throw new TypeError(at + 'Argument 1 is not ' + (object ? 'callable.' : 'an object.'));
    }
    if (options !== undefined && options !== null && typeof options !== 'object') {
      throw new TypeError(at + "Argument 2 can't be converted to a dictionary.");
    }
    hook.queue(function () {
      var deadline = Object.create(IdleDeadline.prototype), s = slots(deadline);
      s.didTimeout = false;
      s.until = hook.until(hook.observe());
      s.open = true;
      try {
        callback.call(undefined, deadline);
      } finally {
        s.open = false;
      }
    }, nextHandle);
    return nextHandle++;
  };
  g.cancelIdleCallback = function cancelIdleCallback(handle) {
    if (!arguments.length) {
      throw new TypeError('Window.cancelIdleCallback: At least 1 argument required, '
        + 'but only 0 passed');
    }
    hook.cancel(handle);
  };
})(globalThis);
