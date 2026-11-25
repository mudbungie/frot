// ElementInternals probe (js.md §15.6): run on Firefox 153.0esr (identity.md
// §3.19 rig) over internals/page.html, its answers kept in firefox153.json.
var out = {};
var t = function (k, f) { try { var v = f(); out[k] = v === undefined ? 'undefined' : v; } catch (e) { out[k] = 'THROW ' + e.name + ': ' + e.message; } };
var $ = function (id) { return document.getElementById(id); };
var nm = function (n) { return n === null ? 'null' : n === undefined ? 'undefined' : n.id || n.localName || n.nodeName; };
var log = [];
var I = new Map();
class Face extends HTMLElement {
  static formAssociated = true;
  constructor() { super(); I.set(this, this.attachInternals()); log.push('ctor ' + this.id); }
  connectedCallback() { log.push('conn ' + this.id); }
  formAssociatedCallback(f) { log.push('assoc ' + this.id + ' ' + nm(f)); }
  formDisabledCallback(d) { log.push('disabled ' + this.id + ' ' + d); }
  formResetCallback() { log.push('reset ' + this.id); }
}
class Plain extends HTMLElement { constructor() { super(); I.set(this, this.attachInternals()); } }
class Off extends HTMLElement { static disabledFeatures = ['internals']; }
t('globals', function () { return [typeof ElementInternals, typeof CustomStateSet, typeof HTMLElement.prototype.attachInternals, HTMLElement.prototype.attachInternals.length, Object.prototype.hasOwnProperty.call(HTMLElement.prototype, 'attachInternals')].join(' '); });
t('proto', function () { return Object.getOwnPropertyNames(ElementInternals.prototype).join(' '); });
t('protoStates', function () { return Object.getOwnPropertyNames(CustomStateSet.prototype).join(' '); });
t('protoValidity', function () { return Object.getOwnPropertyNames(ValidityState.prototype).join(' ') + ' | ' + typeof ValidityState; });
t('newEI', function () { return new ElementInternals(); });
t('newCSS', function () { return new CustomStateSet(); });
t('builtin', function () { return document.createElement('div').attachInternals(); });
t('undef', function () { return document.createElement('x-undef').attachInternals(); });
customElements.define('x-face', Face);
customElements.define('x-plain', Plain);
customElements.define('x-off', Off);
t('parsedLog', function () { return log.join(','); });
t('off', function () { return document.createElement('x-off').attachInternals(); });
t('twice', function () { return $('p1').attachInternals(); });
t('tag', function () { var i = I.get($('p1')); return Object.prototype.toString.call(i) + ' ' + Object.prototype.toString.call(i.states) + ' ' + (i.states === i.states); });
t('form', function () { return ['p1', 'p2', 'p3', 'p4'].map(function (id) { return nm(I.get($(id)).form); }).join(' '); });
t('labels', function () { var l = I.get($('p1')).labels; return Object.prototype.toString.call(l) + ' ' + l.length + ' ' + Array.prototype.map.call(l, nm).join(',') + ' ' + (l === I.get($('p1')).labels); });
t('labels4', function () { return I.get($('p4')).labels.length; });
t('plainForm', function () { return I.get($('pl')).form; });
t('plainLabels', function () { return I.get($('pl')).labels; });
t('plainValue', function () { return I.get($('pl')).setFormValue('x'); });
t('plainWill', function () { return I.get($('pl')).willValidate; });
t('plainValidity', function () { return I.get($('pl')).validity; });
t('plainCheck', function () { return I.get($('pl')).checkValidity(); });
t('plainStates', function () { var s = I.get($('pl')).states; s.add('x'); return s.has('x') + ' ' + $('pl').matches(':state(x)'); });
t('plainRole', function () { var i = I.get($('pl')); var a = [i.role, i.ariaLabel]; i.role = 'button'; i.ariaLabel = 'L'; a.push(i.role, i.ariaLabel, $('pl').getAttribute('role')); i.role = null; a.push(i.role); return a.join(' '); });
t('ariaProps', function () { return Object.getOwnPropertyNames(ElementInternals.prototype).filter(function (k) { return /^(aria|role)/.test(k); }).length; });
t('ariaCoerce', function () { var i = I.get($('pl')); i.ariaLevel = 3; var a = [typeof i.ariaLevel, i.ariaLevel]; i.ariaLevel = undefined; a.push(i.ariaLevel); i.ariaLabel = ''; a.push(JSON.stringify(i.ariaLabel)); return a.join(' '); });
t('shadow', function () { var e = document.createElement('x-plain'); var i = I.get(e); var a = [i.shadowRoot]; var r = e.attachShadow({ mode: 'closed' }); a.push(i.shadowRoot === r); return a.join(' '); });
t('valid0', function () { var i = I.get($('p1')); var v = i.validity; return [i.willValidate, v.valid, v.valueMissing, v.customError, JSON.stringify(i.validationMessage), i.checkValidity(), Object.prototype.toString.call(v), v === i.validity].join(' '); });
t('setValidity', function () {
  var i = I.get($('p1')); var ev = [];
  $('p1').addEventListener('invalid', function (e) { ev.push(e.type + ':' + e.cancelable + ':' + e.bubbles); });
  i.setValidity({ valueMissing: true, customError: true }, 'need');
  var v = i.validity;
  return [v.valid, v.valueMissing, v.customError, v.typeMismatch, i.validationMessage, i.checkValidity(), i.reportValidity(), ev.join('|'), $('p1').matches(':invalid'), $('f').matches(':invalid'), $('p2').matches(':valid')].join(' ');
});
t('noMessage', function () { return I.get($('p1')).setValidity({ tooLong: true }); });
t('anchorBad', function () { return I.get($('p1')).setValidity({ tooLong: true }, 'm', document.body); });
t('anchorOk', function () { var s = document.createElement('span'); $('p1').appendChild(s); var i = I.get($('p1')); i.setValidity({ customError: true }, 'm', s); var a = [i.validity.customError, i.validationMessage]; i.setValidity({}); s.remove(); return a.join(' '); });
t('clear', function () { var i = I.get($('p1')); i.setValidity({}); return [i.validity.valid, JSON.stringify(i.validationMessage), $('p1').matches(':valid')].join(' '); });
t('clearMsg', function () { var i = I.get($('p1')); i.setValidity({}, 'kept?'); return JSON.stringify(i.validationMessage); });
t('setValidity0', function () { return I.get($('p1')).setValidity(); });
t('formValue', function () { var i = I.get($('p1')); return [i.setFormValue('v'), i.setFormValue(null), i.setFormValue('v', 'state')].join(' '); });
t('formValue0', function () { return I.get($('p1')).setFormValue(); });
t('disabled', function () { log.length = 0; $('fs').disabled = true; var a = [log.join(','), I.get($('p2')).willValidate, $('p2').matches(':disabled'), $('p2').matches(':enabled')]; $('fs').disabled = false; a.push(log.join(',')); return a.join(' '); });
t('disabledAttr', function () { log.length = 0; $('p4').setAttribute('disabled', ''); var a = [log.join(','), $('p4').matches(':disabled'), I.get($('p4')).willValidate]; $('p4').removeAttribute('disabled'); a.push(log.join(',')); return a.join(' '); });
t('move', function () { log.length = 0; $('g').appendChild($('p4')); var a = [log.join(','), nm(I.get($('p4')).form)]; document.body.appendChild($('p4')); a.push(log.join(',')); return a.join(' '); });
t('formAttr', function () { log.length = 0; $('p3').setAttribute('form', 'f'); var a = [log.join(','), nm(I.get($('p3')).form)]; $('p3').removeAttribute('form'); a.push(log.join(','), nm(I.get($('p3')).form)); return a.join(' '); });
t('reset', function () { log.length = 0; $('f').reset(); return log.join(','); });
t('states', function () {
  var e = $('p2'), s = I.get(e).states;
  var a = [s.size, s.add('on'), s.size, s.has('on'), e.matches(':state(on)'), document.querySelectorAll(':state(on)').length];
  s.add('on'); s.add('two'); a.push(s.size, Array.from(s).join('+'), [...s.keys()].join('+'), [...s.entries()][0].join('='));
  a.push(s.delete('on'), s.delete('on'), e.matches(':state(on)'), e.matches(':state(two)'));
  s.clear(); a.push(s.size);
  return a.join(' ');
});
t('stateTokens', function () { var e = $('p2'), s = I.get(e).states; var a = []; ['--dash', 'a b', '', 'UP'].forEach(function (k) { try { s.add(k); a.push('ok'); } catch (x) { a.push(x.name); } }); a.push(e.matches(':state(--dash)'), e.matches(':state(UP)'), e.matches(':state(up)')); s.clear(); return a.join(' '); });
t('stateSel', function () { var a = []; [':state(x)', ':state()', ':state(a b)', ':state(--x)'].forEach(function (q) { try { document.querySelector(q); a.push('ok'); } catch (x) { a.push(x.name); } }); return a.join(' '); });
t('stateForEach', function () { var s = I.get($('p2')).states; s.add('q'); var a = []; s.forEach(function (v, k, set) { a.push(v + k + (set === s)); }); s.clear(); return a.join(','); });
t('stateBrand', function () { return CustomStateSet.prototype.has.call({}, 'x'); });
t('internalsBrand', function () { return Object.getOwnPropertyDescriptor(ElementInternals.prototype, 'form').get.call({}); });
t('p5', function () { return [I.get($('p5')).willValidate, $('p5').matches(':disabled'), $('p5').matches(':invalid')].join(' '); });
t('createLog', function () { log.length = 0; var e = document.createElement('x-face'); e.id = 'c'; $('f').appendChild(e); var a = [log.join(',')]; e.remove(); a.push(log.join(',')); return a.join(' | '); });
t('innerHTMLLog', function () { log.length = 0; $('g').innerHTML = '<x-face id=h></x-face>'; return log.join(','); });
t('ctorRead', function () {
  class Early extends HTMLElement { static formAssociated = true; constructor() { super(); var i = this.attachInternals(); this.r = [nm(i.form), i.labels.length, i.willValidate].join(' '); } }
  customElements.define('x-early', Early);
  var d = document.createElement('div'); $('f').appendChild(d); d.innerHTML = '<x-early id=ee></x-early>';
  return $('ee').r + ' ' + nm(I.get($('p1')).form);
});
t('lateFA', function () { class L extends HTMLElement { constructor() { super(); this.i = this.attachInternals(); } } customElements.define('x-late', L); L.formAssociated = true; var e = document.createElement('x-late'); return e.i.form; });
t('getter', function () { class G extends HTMLElement { static get formAssociated() { log.push('read'); return 1; } } log.length = 0; customElements.define('x-getter', G); var a = [log.join(',')]; var e = document.createElement('x-getter'); a.push(nm(e.attachInternals().form)); return a.join(' '); });
resolve(out);
