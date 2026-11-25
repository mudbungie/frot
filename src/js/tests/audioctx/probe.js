// Audio context probe (bl-bd3d): run on Firefox 153.0esr (identity.md §3.19 rig)
// as a page script over audioctx/page.html; its settled answers kept verbatim
// in firefox153.json.
function t(f){try{var v=f();return v===undefined?'undef':v===null?'null':String(v)}catch(e){return 'THROW '+e.name+':'+e.message}}
function chain(C){var r=[],p=C.prototype;while(p&&p!==Object.prototype){r.push(p.constructor.name);p=Object.getPrototypeOf(p)}return r.join('>')+'|'+(Object.getPrototypeOf(C)&&Object.getPrototypeOf(C).name)}
function own(C){return Object.getOwnPropertyNames(C.prototype).join(' ')}
resolve({
 baseChain: t(()=>chain(BaseAudioContext)), acChain: t(()=>chain(AudioContext)), oacChain: t(()=>chain(OfflineAudioContext)), nodeChain: t(()=>chain(AudioNode)), gainChain: t(()=>chain(GainNode)),
 baseOwn: t(()=>own(BaseAudioContext)), acOwn: t(()=>own(AudioContext)), oacOwn: t(()=>own(OfflineAudioContext)), nodeOwn: t(()=>own(AudioNode)),
 complete: (function(){return new Promise(function(res){var c=new OfflineAudioContext(1,128,44100),log=[];c.addEventListener('complete',function(e){log.push('L:'+e.type+':'+(e.target===c)+':'+(e.renderedBuffer?e.renderedBuffer.length:0))});c.oncomplete=function(e){log.push('H')};c.startRendering().then(function(){log.push('P');setTimeout(function(){res(log.join())},50)})})})(),
 statechange: t(()=>{var c=new OfflineAudioContext(1,128,44100);return typeof c.onstatechange+','+(c instanceof EventTarget)+','+typeof c.addEventListener}),
 acInst: t(()=>{var c=new AudioContext();var r=(c instanceof EventTarget)+','+typeof c.onstatechange;c.close();return r}),
});
