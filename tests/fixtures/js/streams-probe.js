// The streams oracle (bl-96f7): every call here was run in Firefox 153.0esr
// (identity.md §3.19 rig, page served over http:) and its answers captured
// verbatim in streams-firefox153.json; js::tests::streams runs the same file in
// frot and requires the same answers. `done(out)` is the page's to define.
var out = {}, P = [];
function E(x) { return x && x.name ? x.name + ': ' + x.message : String(x); }
function T(k, f) { try { out[k] = String(f()); } catch (x) { out[k] = 'THROW ' + E(x); } }
function A(k, f) { P.push(Promise.race([Promise.resolve().then(f), new Promise(function (r) { setTimeout(function () { r('HANG'); }, 1500); })]).then(function (v) { out[k] = typeof v === 'string' ? v : JSON.stringify(v); }, function (x) { out[k] = 'REJ ' + E(x); })); }
function tick(n) { var p = Promise.resolve(); for (var i = 0; i < (n || 10); i++) p = p.then(function () {}); return p; }
var names = ['ReadableStream','ReadableStreamDefaultReader','ReadableStreamDefaultController','ReadableByteStreamController','ReadableStreamBYOBReader','ReadableStreamBYOBRequest','WritableStream','WritableStreamDefaultWriter','WritableStreamDefaultController','TransformStream','TransformStreamDefaultController','ByteLengthQueuingStrategy','CountQueuingStrategy','TextDecoderStream','TextEncoderStream'];
function desc(o, k) { var d = Object.getOwnPropertyDescriptor(o, k); if (!d) return null; var r = (d.get ? 'g' : '') + (d.set ? 's' : '') + ('value' in d ? 'v' : '') + (d.enumerable ? 'E' : '') + (d.writable ? 'W' : '') + (d.configurable ? 'C' : ''); if (typeof d.value === 'function') r += ':' + d.value.length; return r; }
names.forEach(function (n) {
  var C = window[n];
  if (!C) { out['iface ' + n] = 'absent'; return; }
  var P = C.prototype;
  var parent = Object.getPrototypeOf(C);
  var e = { len: C.length, parent: parent && parent.name, statics: Object.getOwnPropertyNames(C).filter(function(k){return ['length','name','prototype'].indexOf(k)<0}),
    proto: Object.getOwnPropertyNames(P).map(function (k) { return k + ' ' + desc(P, k); }),
    syms: Object.getOwnPropertySymbols(P).map(function (s) { return String(s) + ' ' + desc(P, s) + (typeof P[s]==='string'?'='+P[s]:'') }) };
  try { new C(); e.new0 = 'ok'; } catch (x) { e.new0 = x.name + ': ' + x.message; }
  out['iface ' + n] = JSON.stringify(e);
});

T('asyncIterEq', function () { return ReadableStream.prototype[Symbol.asyncIterator] === ReadableStream.prototype.values; });
T('asyncIterName', function () { return ReadableStream.prototype[Symbol.asyncIterator].name; });
T('itproto', function () { var it = new ReadableStream().values(); var p = Object.getPrototypeOf(it); return Object.getOwnPropertyNames(p).join() + '|' + String(p[Symbol.toStringTag]) + '|' + Object.getOwnPropertyNames(Object.getPrototypeOf(p)) + '|' + Object.prototype.toString.call(it) + '|' + (Object.getPrototypeOf(p) === Object.getPrototypeOf(Object.getPrototypeOf(async function*(){}.prototype))); });
T('ctorBad', function () { return new ReadableStream(5); });
T('ctorNull', function () { return new ReadableStream(null); });
T('typeBad', function () { return new ReadableStream({ type: 'x' }); });
T('startNotFn', function () { return new ReadableStream({ start: 5 }); });
T('hwmBad', function () { return new ReadableStream({}, { highWaterMark: -1 }); });
T('hwmNaN', function () { return new ReadableStream({}, { highWaterMark: 'x' }); });
T('sizeBad', function () { return new ReadableStream({}, { size: 5 }); });
T('bytesSize', function () { return new ReadableStream({ type: 'bytes' }, { size: function () { return 1; } }); });
T('startThrows', function () { return new ReadableStream({ start: function () { throw new Error('boom'); } }); });
T('lockedTwice', function () { var r = new ReadableStream(); r.getReader(); return r.getReader(); });
T('modeBad', function () { return new ReadableStream().getReader({ mode: 'x' }); });
T('byobOnDefault', function () { return new ReadableStream().getReader({ mode: 'byob' }); });
T('readerNotStream', function () { return new ReadableStreamDefaultReader({}); });
T('enqueueAfterClose', function () { var c; new ReadableStream({ start: function (x) { c = x; } }); c.close(); return c.enqueue(1); });
T('closeTwice', function () { var c; new ReadableStream({ start: function (x) { c = x; } }); c.close(); return c.close(); });
T('desired', function () { var c; new ReadableStream({ start: function (x) { c = x; } }, { highWaterMark: 3 }); c.enqueue(1); var a = c.desiredSize; c.close(); return a + ',' + c.desiredSize; });
T('bytesEnqueueStr', function () { var c; new ReadableStream({ type: 'bytes', start: function (x) { c = x; } }); return c.enqueue('a'); });
T('bytesEnqueue0', function () { var c; new ReadableStream({ type: 'bytes', start: function (x) { c = x; } }); return c.enqueue(new Uint8Array(0)); });
T('bytesEnqueueDetach', function () { var c; new ReadableStream({ type: 'bytes', start: function (x) { c = x; } }); var u = new Uint8Array(4); c.enqueue(u); return u.byteLength + ',' + u.buffer.byteLength; });
T('wsType', function () { return new WritableStream({ type: 'x' }); });
T('tsType', function () { return new TransformStream({ readableType: 'x' }); });
T('writerLocked', function () { var w = new WritableStream(); w.getWriter(); return w.getWriter(); });
T('cqs', function () { var s = new CountQueuingStrategy({ highWaterMark: 4 }); return s.highWaterMark + ',' + s.size(9) + ',' + s.size.name + ',' + s.size.length + ',' + (s.size === new CountQueuingStrategy({highWaterMark:1}).size) + ',' + Object.getOwnPropertyNames(s); });
T('blqs', function () { var s = new ByteLengthQueuingStrategy({ highWaterMark: 4 }); return s.highWaterMark + ',' + s.size(new Uint8Array(7)) + ',' + s.size.name + ',' + s.size.length + ',' + s.size({}); });
T('cqsBad', function () { return new CountQueuingStrategy({}); });
T('tdsEnc', function () { var t = new TextDecoderStream('latin1', { fatal: true }); return t.encoding + ',' + t.fatal + ',' + t.ignoreBOM; });
T('tdsBad', function () { return new TextDecoderStream('nope'); });
T('tesEnc', function () { return new TextEncoderStream().encoding; });
T('bodyType', function () { var b = new Response('hi').body; return Object.prototype.toString.call(b) + ',' + (b === new Response('x').body) ; });
T('bodySame', function () { var r = new Response('hi'); return r.body === r.body; });
T('bodyNull', function () { return new Response().body + ',' + new Response(null).body; });
T('blobStream', function () { return Object.prototype.toString.call(new Blob(['a']).stream()); });
T('blobProto', function () { return Object.getOwnPropertyNames(Blob.prototype).join(); });
T('respProto', function () { return Object.getOwnPropertyNames(Response.prototype).join(); });
T('fromBad', function () { return ReadableStream.from(5); });
T('fromLen', function () { return ReadableStream.from.length; });
T('pipeToBad', function () { return ReadableStream.prototype.pipeThrough.call(new ReadableStream(), {}); });
A('readAll', function () {
  var r = new ReadableStream({ start: function (c) { c.enqueue('a'); c.enqueue('b'); c.close(); } }).getReader();
  var got = []; function step() { return r.read().then(function (x) { got.push(JSON.stringify(x)); if (!x.done) return step(); }); }
  return step().then(function () { return got.join(' ') + '|' + Object.getOwnPropertyNames(got.length) ; });
});
A('readResultProto', function () { return new ReadableStream({ start: function (c) { c.enqueue(1); } }).getReader().read().then(function (x) { return (Object.getPrototypeOf(x) === Object.prototype) + ',' + Object.keys(x); }); });
A('respBody', function () { var r = new Response('héllo'); var rd = r.body.getReader(); return rd.read().then(function (x) { return Object.prototype.toString.call(x.value) + ',' + x.value.length + ',' + r.bodyUsed; }); });
A('respBodyText', function () { var r = new Response('hi'); r.body.getReader(); return r.text(); });
A('respTextThenBody', function () { var r = new Response('hi'); return r.text().then(function () { return r.bodyUsed + ',' + r.body.locked; }); });
A('byob', function () { var r = new Response('hello world').body.getReader({ mode: 'byob' }); return r.read(new Uint8Array(4)).then(function (x) { return x.done + ',' + Array.from(x.value) + ',' + x.value.buffer.byteLength; }); });
A('byobMin', function () { var r = new Response('hello world').body.getReader({ mode: 'byob' }); return r.read(new Uint8Array(8), { min: 8 }).then(function (x) { return x.done + ',' + x.value.length; }); });
A('tee', function () { var t = new ReadableStream({ start: function (c) { c.enqueue('x'); c.close(); } }).tee(); return Promise.all([t[0].getReader().read(), t[1].getReader().read()]).then(function (v) { return JSON.stringify(v); }); });
A('pipeThroughTES', function () { return new Response('héllo').body.pipeThrough(new TextDecoderStream()).pipeThrough(new TextEncoderStream()).getReader().read().then(function (x) { return Array.from(x.value).join(); }); });
A('transform', function () { var ts = new TransformStream({ transform: function (ch, c) { c.enqueue(ch + ch); } }); var w = ts.writable.getWriter(); w.write('a'); w.close(); return new Response(ts.readable.pipeThrough(new TextEncoderStream())).text(); });
A('asyncIter', function () { var it = ReadableStream.from(['p', 'q']); var s = ''; return (async function () { for await (var v of it) s += v; return s; })(); });
A('cancelReason', function () { var why; var r = new ReadableStream({ cancel: function (x) { why = x; return 'ignored'; } }); return r.cancel('because').then(function (v) { return why + ',' + v; }); });
A('readLocked', function () { var r = new ReadableStream(); var rd = r.getReader(); rd.releaseLock(); return rd.read(); });
A('closedAfterRelease', function () { var r = new ReadableStream(); var rd = r.getReader(); rd.releaseLock(); return rd.closed; });
A('pendingReadRelease', function () { var r = new ReadableStream(); var rd = r.getReader(); var p = rd.read(); rd.releaseLock(); return p; });
A('cancelLocked', function () { var r = new ReadableStream(); r.getReader(); return r.cancel(); });
A('writeClosed', function () { var w = new WritableStream().getWriter(); w.close(); return w.write(1); });
A('writerDesired', function () { var w = new WritableStream({}, { highWaterMark: 2 }).getWriter(); w.write(1); return w.desiredSize + ',' + w.ready.constructor.name; });
A('wsOrder', function () { var log = []; var ws = new WritableStream({ start: function () { log.push('start'); }, write: function (c) { log.push('w' + c); }, close: function () { log.push('close'); } }); var w = ws.getWriter(); w.write(1); w.write(2); return w.close().then(function () { return log.join(); }); });
A('wsAbort', function () { var log = []; var ws = new WritableStream({ abort: function (r) { log.push('abort:' + r); } }); var w = ws.getWriter(); return w.abort('r').then(function () { return w.closed.then(function () { return 'closed'; }, function (e) { return log + '|' + e; }); }); });
A('pipeTo', function () { var got = []; return ReadableStream.from([1, 2, 3]).pipeTo(new WritableStream({ write: function (c) { got.push(c); } })).then(function (v) { return got.join() + ',' + v; }); });
A('pipeToErr', function () { return new ReadableStream({ start: function (c) { c.error(new TypeError('src')); } }).pipeTo(new WritableStream()); });
A('pipeToLocked', function () { var r = new ReadableStream(); r.getReader(); return r.pipeTo(new WritableStream()); });
A('pipeToNotWS', function () { return new ReadableStream().pipeTo({}); });
A('blobStreamRead', function () { return new Blob(['abc']).stream().getReader().read().then(function (x) { return Object.prototype.toString.call(x.value) + Array.from(x.value); }); });
A('terminate', function () { var ts = new TransformStream({ start: function (c) { c.terminate(); } }); return ts.writable.getWriter().write(1); });
A('respFromStream', function () { return new Response(ReadableStream.from(['ab', 'cd'])).text(); });
A('respFromStreamBytes', function () { return new Response(ReadableStream.from([new Uint8Array([104, 105])])).text(); });
A('pullOrder', function () { var log = []; var i = 0; var r = new ReadableStream({ start: function () { log.push('start'); }, pull: function (c) { log.push('pull' + i); c.enqueue(i++); if (i > 3) c.close(); } }, { highWaterMark: 2 });
  return tick().then(function () { log.push('ds'); var rd = r.getReader(); return rd.read().then(function (x) { log.push('r' + x.value); return tick(); }).then(function () { return log.join(); }); }); });
A('wsBackpressure', function () { var log = []; var resolveW; var ws = new WritableStream({ write: function (c) { log.push('w' + c); return new Promise(function (r) { resolveW = r; }); } }, { highWaterMark: 2 }); var w = ws.getWriter();
  log.push('d' + w.desiredSize); w.write(1); log.push('d' + w.desiredSize); w.write(2); log.push('d' + w.desiredSize); w.write(3); log.push('d' + w.desiredSize);
  var readyState = 'pending'; w.ready.then(function () { readyState = 'res'; });
  return tick().then(function () { log.push('ready:' + readyState); resolveW(); return tick(); }).then(function () { log.push('ready:' + readyState + ',d' + w.desiredSize); return log.join(); }); });
A('tsBackpressure', function () { var log = []; var ts = new TransformStream({ transform: function (c, ctl) { log.push('t' + c + ':' + ctl.desiredSize); ctl.enqueue(c); } }); var w = ts.writable.getWriter(); log.push('wd' + w.desiredSize);
  w.write('a'); w.write('b'); return tick().then(function () { log.push('wd' + w.desiredSize); var rd = ts.readable.getReader(); return rd.read(); }).then(function (x) { log.push('r' + x.value); return tick(); }).then(function () { log.push('wd' + w.desiredSize); return log.join(); }); });
A('pipePreventClose', function () { var log = []; var ws = new WritableStream({ write: function (c) { log.push('w' + c); }, close: function () { log.push('close'); } }); return ReadableStream.from(['x']).pipeTo(ws, { preventClose: true }).then(function () { log.push('locked:' + ws.locked); return log.join(); }); });
A('pipeSignal', function () { var ac = new AbortController(); var log = []; var r = new ReadableStream({ cancel: function (x) { log.push('cancel:' + E(x)); } }); var ws = new WritableStream({ abort: function (x) { log.push('abort:' + E(x)); } }); var p = r.pipeTo(ws, { signal: ac.signal }); ac.abort(); return p.then(function () { return 'ok'; }, function (e) { log.push('rej:' + E(e)); return log.join('|'); }); });
A('pipeSignalPre', function () { var ac = new AbortController(); ac.abort('why'); return new ReadableStream().pipeTo(new WritableStream(), { signal: ac.signal }).then(function () { return 'ok'; }, function (e) { return 'rej:' + E(e); }); });
A('pipeDestErr', function () { var log = []; var r = new ReadableStream({ start: function (c) { c.enqueue(1); }, cancel: function (x) { log.push('cancel:' + E(x)); } }); var ws = new WritableStream({ write: function () { throw new Error('wbad'); } }); return r.pipeTo(ws).then(function () { return 'ok'; }, function (e) { log.push('rej:' + E(e)); return log.join('|'); }); });
A('teeCancel', function () { var why; var r = new ReadableStream({ cancel: function (x) { why = x; } }); var t = r.tee(); t[0].cancel('a'); var p = t[1].cancel('b'); return p.then(function () { return JSON.stringify(why) + ',' + r.locked; }); });
A('teeErr', function () { var c; var r = new ReadableStream({ start: function (x) { c = x; } }); var t = r.tee(); c.error(new Error('e1')); return t[0].getReader().read().then(function () { return 'ok'; }, function (e) { return E(e); }); });
A('byobRespond', function () { var r = new ReadableStream({ type: 'bytes', pull: function (c) { var v = c.byobRequest.view; v[0] = 7; v[1] = 8; c.byobRequest.respond(2); } }); var rd = r.getReader({ mode: 'byob' }); return rd.read(new Uint16Array(3)).then(function (x) { return x.value.constructor.name + ',' + x.value.length + ',' + Array.from(new Uint8Array(x.value.buffer)); }); });
A('byobDefaultAuto', function () { var r = new ReadableStream({ type: 'bytes', autoAllocateChunkSize: 5, pull: function (c) { var q = c.byobRequest; q.view[0] = 9; q.respond(1); } }); return r.getReader().read().then(function (x) { return x.value.constructor.name + ',' + x.value.length + ',' + x.value.buffer.byteLength; }); });
A('byobReqNull', function () { var cc; new ReadableStream({ type: 'bytes', start: function (c) { cc = c; } }); return String(cc.byobRequest) + ',' + cc.desiredSize; });
A('byobClosedRead', function () { var r = new ReadableStream({ type: 'bytes', start: function (c) { c.close(); } }); return r.getReader({ mode: 'byob' }).read(new Uint8Array(3)).then(function (x) { return x.done + ',' + x.value.length + ',' + x.value.buffer.byteLength; }); });
A('byobZero', function () { var r = new ReadableStream({ type: 'bytes' }); return r.getReader({ mode: 'byob' }).read(new Uint8Array(0)); });
A('byobNoArg', function () { var r = new ReadableStream({ type: 'bytes' }); return r.getReader({ mode: 'byob' }).read(); });
A('byobMinBig', function () { var r = new ReadableStream({ type: 'bytes' }); return r.getReader({ mode: 'byob' }).read(new Uint8Array(2), { min: 3 }); });
A('byobMin0', function () { var r = new ReadableStream({ type: 'bytes' }); return r.getReader({ mode: 'byob' }).read(new Uint8Array(2), { min: 0 }); });
A('iterReturn', function () { var why = 'none'; var r = new ReadableStream({ start: function (c) { c.enqueue(1); c.enqueue(2); }, cancel: function (x) { why = x; } }); var it = r.values(); return it.next().then(function (a) { return it.return('stop').then(function (b) { return JSON.stringify([a, b]) + ',' + why + ',' + r.locked; }); }); });
A('iterBreak', function () { var r = ReadableStream.from([1, 2, 3]); var s = 0; return (async function () { for await (var v of r) { s += v; break; } return s + ',' + r.locked; })(); });
A('iterPrevent', function () { var r = ReadableStream.from([1, 2, 3]); return (async function () { for await (var v of r.values({ preventCancel: true })) break; var rd = r.getReader(); var x = await rd.read(); return JSON.stringify(x); })(); });
A('writerAbortErr', function () { var ws = new WritableStream({ abort: function () { throw new Error('abort fail'); } }); return ws.abort('x').then(function () { return 'ok'; }, function (e) { return E(e); }); });
A('writerRelease', function () { var ws = new WritableStream(); var w = ws.getWriter(); w.releaseLock(); var a = ws.locked; return w.closed.then(function () { return 'res'; }, function (e) { return a + ',' + E(e); }); });
A('writerReleaseWrite', function () { var ws = new WritableStream(); var w = ws.getWriter(); w.releaseLock(); return w.write(1); });
A('writerReleaseDesired', function () { var ws = new WritableStream(); var w = ws.getWriter(); w.releaseLock(); try { return String(w.desiredSize); } catch (e) { return 'THROW ' + E(e); } });
A('wsCloseLocked', function () { var ws = new WritableStream(); ws.getWriter(); return ws.close(); });
A('wsAbortLocked', function () { var ws = new WritableStream(); ws.getWriter(); return ws.abort(); });
A('ctrlError', function () { var cc; var ws = new WritableStream({ start: function (c) { cc = c; } }); var w = ws.getWriter(); cc.error(new Error('ce')); return w.write(1).then(function () { return 'ok'; }, function (e) { return E(e) + ',' + w.desiredSize; }); });
A('signal', function () { var sig; var ws = new WritableStream({ start: function (c) { sig = c.signal; } }); var a = sig.aborted; ws.abort('rr'); return a + ',' + sig.aborted + ',' + sig.reason + ',' + Object.prototype.toString.call(sig); });
A('tsFlushOrder', function () { var log = []; var ts = new TransformStream({ transform: function (c, k) { k.enqueue(c.toUpperCase()); }, flush: function (k) { log.push('flush'); k.enqueue('!'); } }); var w = ts.writable.getWriter(); w.write('a'); w.close(); return new Response(ts.readable.pipeThrough(new TextEncoderStream())).text().then(function (t) { return t + ',' + log; }); });
A('tsCancel', function () { var log = []; var ts = new TransformStream({ cancel: function (x) { log.push('tc:' + x); } }); return ts.readable.cancel('rc').then(function () { return ts.writable.getWriter().write(1); }).then(function () { return 'ok'; }, function (e) { return log + '|' + E(e); }); });
A('tsTransformThrow', function () { var ts = new TransformStream({ transform: function () { throw new Error('tx'); } }); var w = ts.writable.getWriter(); var p = w.write(1); return p.then(function () { return 'ok'; }, function (e) { return E(e) + ',' + (ts.readable.locked); }).then(function (s) { return ts.readable.getReader().read().then(function () { return s + ',ok'; }, function (e) { return s + ',' + E(e); }); }); });
A('tdsSplit', function () { var t = new TextDecoderStream(); var w = t.writable.getWriter(); w.write(new Uint8Array([0xe2, 0x82])); w.write(new Uint8Array([0xac])); w.close(); return new Response(t.readable.pipeThrough(new TextEncoderStream())).text(); });
A('tesSurrogate', function () { var t = new TextEncoderStream(); var w = t.writable.getWriter(); w.write('\ud83d'); w.write('\ude00'); w.write('\ud83d'); w.close(); var rd = t.readable.getReader(); var parts = []; function s() { return rd.read().then(function (x) { if (x.done) return parts.join('|'); parts.push(Array.from(x.value).join()); return s(); }); } return s(); });
A('respClone', function () { var r = new Response('abc'); var c = r.clone(); return Promise.all([r.text(), c.text()]).then(function (v) { return v.join() + ',' + r.bodyUsed + c.bodyUsed; }); });
A('respCloneUsed', function () { var r = new Response('abc'); return r.text().then(function () { try { r.clone(); return 'ok'; } catch (e) { return E(e); } }); });
A('respTextTwice', function () { var r = new Response('abc'); return r.text().then(function () { return r.text(); }); });
A('respArrayBuffer', function () { return new Response(new Uint8Array([1, 2])).arrayBuffer().then(function (b) { return Object.prototype.toString.call(b) + b.byteLength; }); });
A('respBlob', function () { return new Response('xy').blob().then(function (b) { return b.size + ',' + JSON.stringify(b.type); }); });
A('respEmptyBody', function () { var r = new Response(''); return String(r.body) + ',' + (r.body && r.body.constructor.name); });
A('respNullText', function () { return new Response(null).text().then(function (t) { return JSON.stringify(t) + new Response().bodyUsed; }); });
A('rsLockedStream', function () { var s = new ReadableStream(); s.getReader(); try { new Response(s); return 'ok'; } catch (e) { return E(e); } });
A('respBlobArg', function () { return new Response(new Blob(['qq'])).text(); });
A('closedResolve', function () { var c; var r = new ReadableStream({ start: function (x) { c = x; } }); var rd = r.getReader(); var log = []; rd.closed.then(function (v) { log.push('closed:' + v); }); c.close(); return tick().then(function () { return log.join(); }); });
A('enqueueThenRead', function () { var r = new ReadableStream({ start: function (c) { c.enqueue('a'); c.close(); } }); var rd = r.getReader(); return Promise.all([rd.read(), rd.read(), rd.read()]).then(JSON.stringify); });
A('sizeThrows', function () { var c; var r = new ReadableStream({ start: function (x) { c = x; } }, { size: function () { throw new Error('sz'); } }); try { c.enqueue(1); } catch (e) { return E(e) + ',' + r.getReader().read().then ? E(e) : ''; } });
A('sizeNeg', function () { var c; new ReadableStream({ start: function (x) { c = x; } }, { size: function () { return -1; } }); try { c.enqueue(1); return 'ok'; } catch (e) { return E(e); } });
A('hwmInf', function () { var c; new ReadableStream({ start: function (x) { c = x; } }, { highWaterMark: Infinity }); return String(c.desiredSize); });
A('readerCtorLocked', function () { var r = new ReadableStream(); r.getReader(); try { new ReadableStreamDefaultReader(r); } catch (e) { return E(e); } });
A('byobCtorDefault', function () { try { new ReadableStreamBYOBReader(new ReadableStream()); } catch (e) { return E(e); } });
A('writerCtorBad', function () { try { new WritableStreamDefaultWriter({}); } catch (e) { return E(e); } });
A('brand', function () { try { ReadableStream.prototype.getReader.call({}); } catch (e) { return E(e); } });
A('brandAsync', function () { return ReadableStream.prototype.cancel.call({}); });
A('brandGetter', function () { try { return Object.getOwnPropertyDescriptor(ReadableStream.prototype, 'locked').get.call({}); } catch (e) { return E(e); } });
A('pipeThroughLockedW', function () { var ts = new TransformStream(); ts.writable.getWriter(); try { new ReadableStream().pipeThrough(ts); } catch (e) { return E(e); } });
A('fromAsyncGen', function () { async function* gen() { yield 'a'; yield 'b'; } return new Response(ReadableStream.from(gen()).pipeThrough(new TextEncoderStream())).text(); });
A('fromBadIter', function () { try { ReadableStream.from({}); } catch (e) { return E(e); } });
A('cancelErrored', function () { var r = new ReadableStream({ start: function (c) { c.error('E'); } }); return r.cancel(); });
A('cqsSizeThis', function () { var s = new CountQueuingStrategy({ highWaterMark: 1 }); return Object.getOwnPropertyDescriptor(CountQueuingStrategy.prototype, 'size').get.call({}) === s.size; });
A('breakCustom', function () { var log = []; var it = { next: function () { log.push('next'); return Promise.resolve({ value: 1, done: false }); }, return: function () { log.push('return'); return Promise.resolve({ done: true }); } }; var o = {}; o[Symbol.asyncIterator] = function () { return it; }; return (async function () { for await (var v of o) break; return log.join(); })(); });
A('breakProtoReturn', function () { var log = []; var proto = { next: function () { log.push('next'); return Promise.resolve({ value: 1, done: false }); }, return: function () { log.push('return'); return Promise.resolve({ done: true }); } }; var it = Object.create(proto); var o = {}; o[Symbol.asyncIterator] = function () { return it; }; return (async function () { for await (var v of o) break; return log.join(); })(); });
A('tsCancelRead', function () { var log = []; var tr = {}; Object.defineProperty(tr, 'cancel', { get: function () { log.push('get cancel'); return undefined; } }); ['flush','readableType','start','transform','writableType'].forEach(function (k) { Object.defineProperty(tr, k, { get: function () { log.push('get ' + k); return undefined; } }); }); new TransformStream(tr); return log.join(); });
A('srcRead', function () { var log = []; var tr = {}; ['autoAllocateChunkSize','cancel','pull','start','type'].forEach(function (k) { Object.defineProperty(tr, k, { get: function () { log.push(k); return undefined; } }); }); new ReadableStream(tr); return log.join(); });
A('sinkRead', function () { var log = []; var tr = {}; ['abort','close','start','type','write'].forEach(function (k) { Object.defineProperty(tr, k, { get: function () { log.push(k); return undefined; } }); }); new WritableStream(tr); return log.join(); });
A('respCtype', function () { var r = new Response('x'); return r.headers.get('content-type') + '|' + new Response(new Blob(['a'], { type: 'X/Y' })).headers.get('content-type') + '|' + new Response(new URLSearchParams('a=1')).headers.get('content-type') + '|' + new Response(new Uint8Array(1)).headers.get('content-type'); });

Promise.all(P).then(function () { done(out); });
