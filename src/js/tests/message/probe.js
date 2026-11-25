// postMessage / MessageChannel / BroadcastChannel probe (bl-0af7).
var out = {};
var t = function (k, f) { try { out[k] = f(); } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
function d(P){return Object.getOwnPropertyNames(P).map(function(k){var x=Object.getOwnPropertyDescriptor(P,k);return k+':'+(x.get?'g':'')+(x.set?'s':'')+(('value' in x)?(typeof x.value==='function'?'f':'v'):'')+(x.enumerable?'e':'')+(x.configurable?'c':'')+(x.writable?'w':'');}).join(' ');}
var L = [];
function ev(tag, e) { var p = e.ports; L.push(tag + ':' + JSON.stringify(e.data) + '|' + e.origin + '|' + (e.source === window ? 'win' : e.source === null ? 'null' : e.source && e.source === (window.FW || {}) ? 'frame' : String(e.source)) + '|' + JSON.stringify(e.lastEventId) + '|' + (Array.isArray(p) ? 'arr' : typeof p) + p.length + Object.isFrozen(p) + '|' + e.isTrusted + e.bubbles + e.cancelable + '|' + (e.target === window ? 'win' : e.target && e.target.constructor.name) + '|' + e.constructor.name); }
t('shapes', function () { return [d(MessageEvent.prototype), d(MessageChannel.prototype), d(MessagePort.prototype), d(BroadcastChannel.prototype), MessageEvent.length, MessageChannel.length, BroadcastChannel.length, typeof window.postMessage, window.hasOwnProperty('postMessage'), window.postMessage.length, d(Object.getPrototypeOf(MessagePort.prototype)).slice(0, 40), d(Object.getPrototypeOf(BroadcastChannel.prototype)).slice(0, 40)].join(' || '); });
t('ctor', function () { var p = new MessageChannel().port1; var e = new MessageEvent('message', { data: { a: 1 }, origin: 'o', lastEventId: 'l', source: window, ports: [p] }); return [JSON.stringify(e.data), e.origin, e.lastEventId, e.source === window, e.ports.length, e.ports[0] === p, e.ports === e.ports, Object.isFrozen(e.ports)].join(); });
t('ctorDefaults', function () { var e = new MessageEvent('x'); return [e.data, e.origin, e.lastEventId, e.source, e.ports.length, e.bubbles].join('/'); });
t('ctorBadSource', function () { return new MessageEvent('x', { source: {} }); });
t('pmNoArgs', function () { window.postMessage(); });
t('pmOneArg', function () { window.postMessage(1); return 'ok'; });
t('pmBadOrigin', function () { window.postMessage(1, 'nope'); });
t('pmFn', function () { window.postMessage(function () {}, '*'); });
t('pmSym', function () { window.postMessage(Symbol(), '*'); });
var syncDone = false;
window.addEventListener('message', function (e) { ev('W' + (syncDone ? '' : 'sync'), e); });
setTimeout(function () { L.push('timeout0'); }, 0);
Promise.resolve().then(function () { L.push('micro'); });
var obj = { a: [1, 2], d: new Date(0), m: new Map([[1, 2]]) };
window.postMessage(obj, '*');
window.postMessage('slash', '/');
window.postMessage('exact', location.origin);
window.postMessage('other', 'http://example.com');
window.postMessage('opts', { targetOrigin: '*' });
window.postMessage('optsDefault', {});
window.postMessage('optsBad', { targetOrigin: 'http://example.com' });
var cloned;
window.addEventListener('message', function (e) { if (e.data && e.data.a) cloned = [e.data === obj, e.data.d instanceof Date, e.data.m instanceof Map, e.data.m.get(1)].join(); }, { once: true });
// channels
var ch = new MessageChannel();
ch.port2.onmessage = function (e) { ev('P2', e); };
ch.port1.postMessage({ x: 1 });
var ch2 = new MessageChannel();
ch2.port2.addEventListener('message', function (e) { ev('P2b', e); });
ch2.port1.postMessage('queued-before-start');
setTimeout(function () { ch2.port2.start(); ch2.port1.postMessage('after-start'); }, 20);
var ch3 = new MessageChannel();
ch3.port2.onmessage = function (e) { ev('P3', e); };
ch3.port1.postMessage('before-close'); ch3.port1.close(); ch3.port1.postMessage('after-close');
var ch4 = new MessageChannel();
ch4.port1.onmessage = function (e) { ev('P4', e); };
ch4.port2.postMessage('back');
t('portSelf', function () { var c = new MessageChannel(); c.port1.postMessage(1, [c.port1]); });
var ch5 = new MessageChannel(); var ch6 = new MessageChannel();
ch5.port2.onmessage = function (e) { ev('P5', e); e.ports[0].postMessage('via-transferred'); };
ch6.port2.onmessage = function (e) { ev('P6', e); };
ch5.port1.postMessage('carry', [ch6.port1]);
t('transferred', function () { var c = new MessageChannel(), c2 = new MessageChannel(); c.port1.postMessage(1, [c2.port1]); try { c2.port1.postMessage(2); return 'posted-on-neutered'; } catch (e) { return e.name; } });
// broadcast
var b1 = new BroadcastChannel('chan'), b2 = new BroadcastChannel('chan'), b3 = new BroadcastChannel('other');
b1.onmessage = function (e) { ev('B1', e); };
b2.onmessage = function (e) { ev('B2', e); };
b3.onmessage = function (e) { ev('B3', e); };
b1.postMessage({ hi: 1 });
t('bcName', function () { return b1.name + ',' + new BroadcastChannel(5).name; });
t('bcClosed', function () { var b = new BroadcastChannel('chan'); b.close(); b.postMessage(1); });
t('bcNoArg', function () { return new BroadcastChannel(); });
// frames
var f = document.createElement('iframe'); document.body.appendChild(f); var FW = window.FW = f.contentWindow;
FW.addEventListener('message', function (e) { ev('F', e); });
FW.postMessage('to-frame', '*');
FW.eval("parent.postMessage('from-frame', '*')");
var fb = new FW.BroadcastChannel('chan'); fb.onmessage = function (e) { ev('FB', e); };
b2.postMessage('to-all');
syncDone = true;
L.push('sync-end');
setTimeout(function () { out.log = L.join('\n'); out.cloned = cloned; resolve(out); }, 300);
