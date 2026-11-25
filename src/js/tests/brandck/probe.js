// Receiver-check probe (bl-a1a1): run on Firefox 153.0esr (identity.md §3.19 rig)
// as a page script over brandck/page.html; its answers kept verbatim in
// firefox153.json.
function t(f){try{var v=f();return v===undefined?'undef':v===null?'null':String(v)}catch(e){return 'THROW '+e.name+':'+e.message}}
function G(P,k){return Object.getOwnPropertyDescriptor(P,k).get} function S(P,k){return Object.getOwnPropertyDescriptor(P,k).set}
var b=document.body, tx=document.getElementById('p').firstChild;
resolve({
 cdLen: t(()=>G(CharacterData.prototype,'length').call(b)),
 nodeName: t(()=>G(Node.prototype,'nodeName').call({})),
 tagName: t(()=>G(Element.prototype,'tagName').call(tx)),
 dtName: t(()=>G(DocumentType.prototype,'name').call(b)),
 protoRead: t(()=>Node.prototype.nodeType),
 protoTitle: t(()=>HTMLElement.prototype.title),
 setId: t(()=>S(Element.prototype,'id').call(tx,'x')),
 method: t(()=>Node.prototype.appendChild.call({}, b)),
 methodUndef: t(()=>{var f=document.getElementById;return f('p')}),
 methodNull: t(()=>Node.prototype.hasChildNodes.call(null)),
 aelBare: t(()=>{var f=EventTarget.prototype.addEventListener;f.call(undefined,'x',function(){});return 'ok'}),
 docOnElem: t(()=>Document.prototype.createElement.call(b,'i')),
 htmlOnSvg: t(()=>G(HTMLElement.prototype,'offsetTop').call(document.createElementNS('http://www.w3.org/2000/svg','g'))),
 okCall: t(()=>G(Node.prototype,'nodeName').call(tx)),
 blobSize: t(()=>G(Blob.prototype,'size').call({})),
 urlHref: t(()=>G(URL.prototype,'href').call({})),
 styleProto: t(()=>CSSStyleDeclaration.prototype.cssText),
 navProto: t(()=>Navigator.prototype.userAgent),
 locProto: t(()=>typeof Location.prototype.href),
 storage: t(()=>Storage.prototype.getItem.call({},'a')),
 evType: t(()=>Event.prototype.type),
 histLen: t(()=>History.prototype.length),
 screenW: t(()=>Screen.prototype.width),
 perfNow: t(()=>Performance.prototype.now.call({})),
 headersGet: t(()=>Headers.prototype.get.call({},'a')),
 respJson: t(()=>Response.prototype.json.call({})),
});
