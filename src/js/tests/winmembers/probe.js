// Window members probe (bl-d8f7): run on Firefox 153.0esr (identity.md §3.19 rig)
// as a page script over winmembers/page.html; its answers kept verbatim in
// firefox153.json.
function t(f){try{var v=f();return v===undefined?'undef':v===null?'null':typeof v==='object'?Object.prototype.toString.call(v):String(v)}catch(e){return 'THROW '+e.name+':'+e.message}}
function d(o,k){var x=Object.getOwnPropertyDescriptor(o,k);if(!x)return 'none';return (x.get?'g':'')+(x.set?'s':'')+('value' in x?(typeof x.value==='function'?'f'+x.value.length:'v'):'')+(x.enumerable?'e':'')+(x.configurable?'c':'')+(x.writable?'w':'')}
var r={};
['name','origin','opener','event','scroll','scrollTo','scrollBy','focus','blur','stop','print','alert','confirm','prompt','open','close','status','closed','structuredClone','getSelection','scrollX','scrollY','pageXOffset'].forEach(function(k){r['d_'+k]=d(window,k)});
r.vals=t(()=>JSON.stringify([name,origin===location.origin,opener,typeof event,status,closed]));
r.setName=t(()=>{name=5;var v=name;name='';return typeof v+':'+v});
r.setOrigin=t(()=>{window.origin='x';var v=origin;return v+','+Object.keys(Object.getOwnPropertyDescriptor(window,'origin')).join('/')});
r.eventDuring=t(()=>{var o=[];var p=document.getElementById('p');p.addEventListener('click',function(e){o.push(window.event===e)});p.dispatchEvent(new Event('click'));o.push(window.event);return o.join()});
r.scrollRet=t(()=>[scrollTo(0,100),scrollY,scroll({top:5}),scrollBy(0,1),scrollX].join());
r.calls=t(()=>[focus(),blur(),stop(),String(close())].join());
r.alertLen=t(()=>[alert.length,confirm.length,prompt.length,open.length,print.length,scrollTo.length,scrollBy.length,scroll.length].join());
r.sc=t(()=>{var a={x:[1,{y:2}]};var b=structuredClone(a);return JSON.stringify(b)+(b!==a)});
r.scFn=t(()=>structuredClone(function(){}));
r.sel=t(()=>{var s=getSelection();return [Object.prototype.toString.call(s),s.rangeCount,String(s),s.type,s===document.getSelection(),s===getSelection()].join()});
r.selProto=t(()=>Object.getOwnPropertyNames(Selection.prototype).join(' '));
r.selVals=t(()=>{var s=getSelection();return [s.anchorNode,s.anchorOffset,s.focusNode,s.focusOffset,s.isCollapsed,s.direction].join()});
r.getRangeAt=t(()=>getSelection().getRangeAt(0));
r.openNoGesture=t(()=>open('about:blank'));
r.dialogs=t(()=>[typeof alert,typeof confirm,typeof prompt].join());
resolve(r);
