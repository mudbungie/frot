//! The rejection table, row for row as read off Firefox 153.0esr (identity.md
//! §3.19) — and the one place frot answers differently on purpose: what it has
//! not implemented rejects `NotSupportedError` (identity.md §11).

use super::{page, settle, ACCESS, DATA, NOT_SUPPORTED, SYNTAX};

#[test]
fn import_key_rejects_as_firefox_does() {
    let s = page();
    for (call, want) in [
        (
            "S.importKey('foo',U(16),'AES-GCM',false,['encrypt'])",
            SYNTAX,
        ),
        (
            "S.importKey('RAW',U(16),'AES-GCM',false,['encrypt'])",
            SYNTAX,
        ),
        ("S.importKey('raw',U(5),'AES-GCM',false,['encrypt'])", DATA),
        ("S.importKey('raw',U(0),H256,false,['sign'])", DATA),
        ("S.importKey('raw',U(16),'AES-GCM',false,['sign'])", SYNTAX),
        ("S.importKey('raw',U(16),'AES-GCM',false,['bogus'])", SYNTAX),
        ("S.importKey('raw',U(16),'AES-GCM',false,[])", SYNTAX),
        ("S.importKey('raw',U(16),'HMAC',false,['sign'])", SYNTAX),
        (
            "S.importKey('raw',U(16),{name:'HMAC',hash:'MD5'},false,['sign'])",
            SYNTAX,
        ),
        ("S.importKey('raw',U(16),H256,false,['encrypt'])", SYNTAX),
        ("S.importKey('raw',U(16),H256,false,[])", SYNTAX),
        (
            "S.importKey('raw',{kty:'oct',k:'AAAA'},'AES-GCM',false,['encrypt'])",
            SYNTAX,
        ),
        (
            "S.importKey('spki',U(16),'AES-GCM',false,['encrypt'])",
            NOT_SUPPORTED,
        ),
        (
            "S.importKey('pkcs8',U(16),H256,false,['sign'])",
            NOT_SUPPORTED,
        ),
        (
            "S.importKey('raw',U(16),'FOO',false,['encrypt'])",
            NOT_SUPPORTED,
        ),
        (
            "S.importKey('raw',U(16),'SHA-256',false,['encrypt'])",
            NOT_SUPPORTED,
        ),
        (
            "S.importKey('raw',U(16),null,false,['encrypt'])",
            NOT_SUPPORTED,
        ),
        // The order of the usage and data checks differs by algorithm (measured).
        ("S.importKey('raw',U(5),'AES-GCM',true,['sign'])", SYNTAX),
        (
            "S.importKey('raw',U(0),{name:'HMAC',hash:'SHA-1'},true,['encrypt'])",
            DATA,
        ),
    ] {
        assert_eq!(settle(&s, call), want, "{call}");
    }
    for (call, want) in [
        (
            "S.importKey('raw',U(16),'AES-GCM')",
            "SubtleCrypto.importKey: At least 5 arguments required, but only 3 passed",
        ),
        (
            "S.importKey('raw','abc','AES-GCM',false,['encrypt'])",
            "SubtleCrypto.importKey: Argument 2 is not an object.",
        ),
        (
            "S.importKey('jwk',null,'AES-GCM',false,['encrypt'])",
            "SubtleCrypto.importKey: Argument 2 is not an object.",
        ),
        (
            "S.importKey('raw',U(16),'AES-GCM',false,'encrypt')",
            "SubtleCrypto.importKey: Argument 5 can't be converted to a sequence.",
        ),
        (
            "S.importKey('raw',U(16),{},false,['encrypt'])",
            "The operation could not be performed.",
        ),
    ] {
        assert_eq!(settle(&s, call), format!("TypeError: {want}"), "{call}");
    }
    // A 24-byte key and an ArrayBuffer are both fine.
    assert_eq!(
        settle(
            &s,
            "S.importKey('raw',new ArrayBuffer(24),'AES-GCM',false,['encrypt']).then(function(k){\
             return k.algorithm.length})"
        ),
        "192"
    );
}

#[test]
fn a_key_only_does_what_it_was_made_for() {
    let s = page();
    let with = |keys: &str, call: &str| {
        settle(
            &s,
            &format!(
                "Promise.all([aes(['encrypt','decrypt']),aes(['encrypt'],true),hm(['sign']),\
                 hm(['verify'],true),{keys}]).then(function(k){{var a=k[0],ax=k[1],h=k[2],\
                 hv=k[3],x=k[4];return {call}}})"
            ),
        )
    };
    for call in [
        "S.encrypt(GCM,h,U(4))",
        "S.decrypt(GCM,ax,U(20))",
        "S.encrypt('HMAC',h,U(4))",
        "S.encrypt({name:'AES-CBC',iv:U(16)},a,U(4))",
        "S.sign('HMAC',hv,U(4))",
        "S.sign('HMAC',a,U(4))",
        "S.sign('AES-GCM',a,U(4))",
        "S.verify('HMAC',h,U(32),U(4))",
        "S.deriveBits('AES-GCM',a,8)",
        "S.deriveBits('FOO',a,8)",
        "S.deriveKey('HMAC',h,'AES-GCM',true,['encrypt'])",
        "S.wrapKey('raw',ax,a,GCM)",
        "S.unwrapKey('raw',U(32),a,GCM,'AES-GCM',true,['encrypt'])",
    ] {
        assert_eq!(with("0", call), ACCESS, "{call}");
    }
    for call in [
        "S.encrypt({name:'FOO'},a,U(4))",
        "S.encrypt(null,a,U(4))",
        "S.encrypt('SHA-256',a,U(4))",
        "S.sign('FOO',h,U(4))",
        "S.sign('SHA-256',h,U(4))",
    ] {
        assert_eq!(with("0", call), NOT_SUPPORTED, "{call}");
    }
    for (call, want) in [
        (
            "S.encrypt({},a,U(4))",
            "The operation could not be performed.",
        ),
        (
            "S.encrypt(GCM,{},U(4))",
            "SubtleCrypto.encrypt: Argument 2 does not implement interface CryptoKey.",
        ),
        (
            "S.decrypt(GCM,Object.create(CryptoKey.prototype),U(4))",
            "SubtleCrypto.decrypt: Argument 2 does not implement interface CryptoKey.",
        ),
        (
            "S.encrypt(GCM,a,'abc')",
            "SubtleCrypto.encrypt: Argument 3 could not be converted to any of: \
             ArrayBufferView, ArrayBuffer.",
        ),
        (
            "S.verify('HMAC',hv,'sig',U(4))",
            "SubtleCrypto.verify: Argument 3 could not be converted to any of: \
             ArrayBufferView, ArrayBuffer.",
        ),
        (
            "S.verify('HMAC',hv,U(4))",
            "SubtleCrypto.verify: At least 4 arguments required, but only 3 passed",
        ),
        (
            "S.wrapKey('raw',{},a,GCM)",
            "SubtleCrypto.wrapKey: Argument 2 does not implement interface CryptoKey.",
        ),
        (
            "S.deriveBits('HKDF',{},8)",
            "SubtleCrypto.deriveBits: Argument 2 does not implement interface CryptoKey.",
        ),
        (
            "S.deriveKey('HKDF',5,'AES-GCM',true,['encrypt'])",
            "SubtleCrypto.deriveKey: Argument 2 does not implement interface CryptoKey.",
        ),
    ] {
        assert_eq!(with("0", call), format!("TypeError: {want}"), "{call}");
    }
}

/// Where frot stops short of Firefox it says so: every one of these SUCCEEDS on
/// 153.0esr and rejects `NotSupportedError` here — never a wrong answer.
#[test]
fn what_is_not_implemented_rejects_not_supported() {
    let s = page();
    for alg in [
        "'PBKDF2'",
        "'HKDF'",
        "'AES-CBC'",
        "'AES-CTR'",
        "'AES-KW'",
        "'Ed25519'",
        "'X25519'",
        "{name:'ECDSA',namedCurve:'P-256'}",
        "{name:'RSA-OAEP',hash:'SHA-256'}",
    ] {
        assert_eq!(
            settle(
                &s,
                &format!("S.importKey('raw',U(32),{alg},false,['deriveBits'])")
            ),
            NOT_SUPPORTED,
            "{alg}"
        );
        assert_eq!(
            settle(&s, &format!("S.generateKey({alg},false,['sign'])")),
            NOT_SUPPORTED,
            "{alg}"
        );
    }
    let wrapper = "aes(['encrypt','decrypt','wrapKey','unwrapKey'])";
    for call in [
        "S.encrypt({name:'AES-GCM',iv:U(16)},k,U(4))",
        "S.encrypt({name:'AES-GCM',iv:U(1)},k,U(4))",
        "S.encrypt({name:'AES-GCM',iv:U(12),tagLength:96},k,U(4))",
        "S.decrypt({name:'AES-GCM',iv:U(12),tagLength:'32'},k,U(20))",
        "aes(['encrypt'],true).then(function(x){return S.wrapKey('raw',x,k,GCM)})",
        "S.unwrapKey('raw',U(32),k,GCM,'AES-GCM',true,['encrypt'])",
    ] {
        let expr = format!("{wrapper}.then(function(k){{return {call}}})");
        assert_eq!(settle(&s, &expr), NOT_SUPPORTED, "{call}");
    }
    assert_eq!(
        settle(
            &s,
            &format!("{wrapper}.then(function(k){{return S.unwrapKey('raw','x',k,GCM)}})")
        ),
        "TypeError: SubtleCrypto.unwrapKey: At least 7 arguments required, but only 4 passed"
    );
    assert_eq!(
        settle(
            &s,
            &format!(
                "{wrapper}.then(function(k){{return S.unwrapKey('raw','x',k,GCM,'AES-GCM',\
                 true,[])}})"
            )
        ),
        "TypeError: SubtleCrypto.unwrapKey: Argument 2 could not be converted to any of: \
         ArrayBufferView, ArrayBuffer."
    );
}
