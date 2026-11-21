//! `crypto.subtle` as a page sees it (`bl-0647`): the Firefox 153.0esr shape,
//! the two keyed algorithms end to end, and the measured error table
//! (identity.md §3.19). The primitives' known answers are pinned in
//! `syscall::subtle::tests`; here they are reached the way a page reaches them.

use super::{sess, Session};

const HELPERS: &str = "\
function hex(b){return Array.from(new Uint8Array(b),function(x){return (x+256).toString(16)\
.slice(1)}).join('')}\
function U(n){return new Uint8Array(n)}\
function unhex(s){return Uint8Array.from(s.match(/../g)||[],function(h){return parseInt(h,16)})}\
var S=crypto.subtle, GCM={name:'AES-GCM',iv:U(12)}, H256={name:'HMAC',hash:'SHA-256'};\
function aes(usages,ext,n){return S.importKey('raw',U(n||32),'AES-GCM',!!ext,usages)}\
function hm(usages,ext){return S.importKey('raw',U(32),H256,!!ext,usages)}";

fn page() -> Session {
    let s = sess("<html><body></body></html>");
    s.eval(HELPERS).unwrap();
    s
}

/// What the promise `expr` settles to: its value, or `Name: message`.
fn settle(s: &Session, expr: &str) -> String {
    s.eval(&format!(
        "var __out='pending';Promise.resolve().then(function(){{return {expr}}}).then(\
         function(v){{__out=String(v)}},function(e){{__out=e.name+': '+e.message}});0"
    ))
    .unwrap();
    s.eval("__out").unwrap()
}

const NOT_SUPPORTED: &str = "NotSupportedError: Operation is not supported";
const SYNTAX: &str = "SyntaxError: An invalid or illegal string was specified";
const DATA: &str = "DataError: Data provided to an operation does not meet requirements";
const ACCESS: &str = "InvalidAccessError: A parameter or an operation is not supported by the \
                      underlying object";
const OPERATION: &str = "OperationError: The operation failed for an operation-specific reason";

#[test]
fn the_surface_is_firefox_shaped() {
    let s = page();
    let eval = |src: &str| s.eval(src).unwrap();
    assert_eq!(
        eval("Object.getOwnPropertyNames(Crypto.prototype).join()"),
        "getRandomValues,randomUUID,subtle,constructor"
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(SubtleCrypto.prototype).join()"),
        "encrypt,decrypt,sign,verify,digest,generateKey,deriveKey,deriveBits,importKey,\
         exportKey,wrapKey,unwrapKey,constructor"
    );
    assert_eq!(
        eval(
            "Object.getOwnPropertyNames(SubtleCrypto.prototype).slice(0,12).map(function(k){\
             return SubtleCrypto.prototype[k].length}).join()"
        ),
        "3,3,3,4,2,3,5,2,5,2,4,7"
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(CryptoKey.prototype).join()"),
        "type,extractable,algorithm,usages,constructor"
    );
    assert_eq!(
        eval(
            "[String(crypto.subtle), crypto.subtle===crypto.subtle, \
             Object.getOwnPropertyNames(crypto.subtle).length, \
             S.digest.name, S.digest.toString()==='function digest() {\\n    [native code]\\n}', \
             Object.getOwnPropertyDescriptor(Crypto.prototype,'subtle').get.name, \
             Object.getOwnPropertyDescriptor(Crypto.prototype,'subtle').get.toString()\
             .slice(0,17), typeof __frot_subtle].join()"
        ),
        "[object SubtleCrypto],true,0,digest,true,get subtle,function subtle(),undefined"
    );
    for ctor in ["SubtleCrypto", "CryptoKey"] {
        assert_eq!(
            eval(&format!(
                "(function(){{try{{new {ctor}()}}catch(e){{return e.name+': '+e.message}}}})()"
            )),
            "TypeError: Illegal constructor."
        );
    }
    // A method always returns a Promise — a WebIDL failure rejects, never throws.
    assert_eq!(eval("S.digest() instanceof Promise"), "true");
    assert_eq!(
        settle(&s, "S.digest()"),
        "TypeError: SubtleCrypto.digest: At least 2 arguments required, but only 0 passed"
    );
    assert_eq!(
        settle(&s, "SubtleCrypto.prototype.digest.call({}, 'SHA-1', U(1))"),
        "TypeError: 'digest' called on an object that does not implement interface SubtleCrypto."
    );
    assert_eq!(
        eval(
            "(function(){try{Object.getOwnPropertyDescriptor(Crypto.prototype,'subtle').get\
             .call({})}catch(e){return e.message}})()"
        ),
        "'get subtle' called on an object that does not implement interface Crypto."
    );
}

/// `[SecureContext]`: an insecure page has no `subtle`, no interfaces, and no
/// leftover hand-off global.
#[test]
fn an_insecure_page_has_none_of_it() {
    let s = Session::new(
        crate::dom::Document::parse("<html><body></body></html>"),
        super::StyleSource::Bare,
        super::Env {
            url: "http://example.com/".into(),
            ..super::test_env()
        },
        &crate::fetch::FetchSession::new(Vec::new()),
    );
    assert_eq!(
        s.eval(
            "[typeof crypto.subtle, 'subtle' in crypto, typeof SubtleCrypto, typeof CryptoKey, \
             typeof __frot_subtle].join()"
        )
        .unwrap(),
        "undefined,false,undefined,undefined,undefined"
    );
}

#[test]
fn digest_takes_every_buffer_shape_and_refuses_what_is_not_a_digest() {
    let s = page();
    assert_eq!(
        settle(
            &s,
            "S.digest('sha-256', new Uint8Array([97,98,99])).then(hex)"
        ),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // An ArrayBuffer, a DataView, a view over part of a buffer, a dictionary.
    for data in [
        "new Uint8Array([97,98,99]).buffer",
        "new DataView(new Uint8Array([97,98,99]).buffer)",
        "new Uint8Array([0,97,98,99,0]).subarray(1,4)",
    ] {
        assert_eq!(
            settle(&s, &format!("S.digest({{name:'SHA-1'}}, {data}).then(hex)")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }
    assert_eq!(
        settle(
            &s,
            "S.digest('SHA-384', U(0)).then(function(b){return String(b)+b.byteLength})"
        ),
        "[object ArrayBuffer]48"
    );
    assert_eq!(
        settle(
            &s,
            "S.digest('SHA-512', U(0)).then(function(b){return b.byteLength})"
        ),
        "64"
    );
    for alg in [
        "'MD5'",
        "{name:'FOO'}",
        "5",
        "null",
        "'AES-GCM'",
        "'SHA-224'",
    ] {
        assert_eq!(settle(&s, &format!("S.digest({alg}, U(1))")), NOT_SUPPORTED);
    }
    assert_eq!(
        settle(&s, "S.digest({}, U(1))"),
        "TypeError: The operation could not be performed."
    );
    assert_eq!(
        settle(&s, "S.digest('SHA-256', 'abc')"),
        "TypeError: SubtleCrypto.digest: Argument 2 could not be converted to any of: \
         ArrayBufferView, ArrayBuffer."
    );
}

#[test]
fn aes_gcm_keys_encrypt_and_decrypt() {
    let s = page();
    // The GCM specification's test case 14, through the page's own calls.
    assert_eq!(
        settle(
            &s,
            "aes(['encrypt']).then(function(k){return S.encrypt(GCM,k,U(16))}).then(hex)"
        ),
        "cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919"
    );
    // Test case 4: associated data, and a round trip back to the plaintext.
    s.eval(
        "var P='d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39'",
    )
    .unwrap();
    assert_eq!(
        settle(
            &s,
            "S.importKey('raw',unhex('feffe9928665731c6d6a8f9467308308'),{name:'aes-gcm'},false,\
             ['decrypt','encrypt']).then(function(k){var a={name:'AES-GCM',\
             iv:unhex('cafebabefacedbaddecaf888'),tagLength:128,\
             additionalData:unhex('feedfacedeadbeeffeedfacedeadbeefabaddad2').buffer};\
             return S.encrypt(a,k,unhex(P)).then(function(c){\
             return S.decrypt(a,k,c).then(function(p){return hex(c)+' '+(hex(p)===P)})})})"
        ),
        "42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e\
         21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091\
         5bc94fbc3221a5db94fae95ae7121a47 true"
    );
    assert_eq!(
        settle(
            &s,
            "aes(['decrypt','encrypt','unwrapKey','wrapKey','encrypt'],true,16).then(function(k){\
             return [String(k),Object.getOwnPropertyNames(k).length,k.type,k.extractable,\
             JSON.stringify(k.algorithm),k.usages.join(),k.algorithm===k.algorithm,\
             k.usages===k.usages].join(' ')})"
        ),
        "[object CryptoKey] 0 secret true {\"name\":\"AES-GCM\",\"length\":128} \
         encrypt,decrypt,wrapKey,unwrapKey true true"
    );
    // Authentication failures, and the parameters Firefox rejects.
    for bad in [
        "S.decrypt(GCM,k,U(20))",
        "S.decrypt(GCM,k,U(4))",
        "S.encrypt({name:'AES-GCM'},k,U(4))",
        "S.encrypt('AES-GCM',k,U(4))",
        "S.encrypt({name:'AES-GCM',iv:U(0)},k,U(4))",
        "S.encrypt({name:'AES-GCM',iv:'abcdefghijkl'},k,U(4))",
        "S.encrypt({name:'AES-GCM',iv:U(12),tagLength:33},k,U(4))",
        "S.encrypt({name:'AES-GCM',iv:U(12),tagLength:136},k,U(4))",
        "S.encrypt({name:'AES-GCM',iv:U(12),additionalData:null},k,U(4))",
    ] {
        let expr = format!("aes(['encrypt','decrypt']).then(function(k){{return {bad}}})");
        assert_eq!(settle(&s, &expr), OPERATION, "{bad}");
    }
    assert_eq!(
        settle(
            &s,
            "aes(['encrypt','decrypt']).then(function(k){return S.encrypt(GCM,k,U(0)).then(\
             function(c){return S.decrypt(GCM,k,c).then(function(p){return c.byteLength+'/'+\
             p.byteLength})})})"
        ),
        "16/0"
    );
}

#[test]
fn hmac_keys_sign_and_verify() {
    let s = page();
    // RFC 4231 test case 1, and the key's own hash winning over the call's.
    assert_eq!(
        settle(
            &s,
            "S.importKey('raw',U(20).fill(11),{name:'hmac',hash:{name:'sha-512'}},false,['sign'])\
             .then(function(k){return S.sign({name:'HMAC',hash:'SHA-1'},k,\
             Uint8Array.from('Hi There',function(c){return c.charCodeAt(0)}))}).then(hex)"
        ),
        "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde\
         daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
    );
    assert_eq!(
        settle(
            &s,
            "hm(['verify','sign']).then(function(k){return S.sign('HMAC',k,U(3)).then(\
             function(sig){return Promise.all([S.verify('HMAC',k,sig,U(3)),\
             S.verify({name:'HMAC'},k,sig,U(4)),S.verify('HMAC',k,sig.slice(1),U(3)),\
             S.verify('HMAC',k,U(0),U(3)),JSON.stringify(k.algorithm),k.usages.join()])})})\
             .then(function(a){return a.join(' ')})"
        ),
        "true false false false {\"name\":\"HMAC\",\"hash\":{\"name\":\"SHA-256\"},\"length\":256} \
         sign,verify"
    );
    // A caller's `length` is ignored on import: the key's own bits are reported.
    assert_eq!(
        settle(
            &s,
            "S.importKey('raw',U(20),{name:'HMAC',hash:'SHA-1',length:157},true,['sign'])\
             .then(function(k){return k.algorithm.length})"
        ),
        "160"
    );
}

mod errors;
mod firefox;
mod keys;
