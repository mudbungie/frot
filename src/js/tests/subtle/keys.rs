//! Keys in and out: `raw` and `jwk` import/export and `generateKey`, with the
//! shapes Firefox 153.0esr produces (identity.md §3.19).

use super::{page, settle, ACCESS, DATA, NOT_SUPPORTED, OPERATION, SYNTAX};

const K16: &str = "AAAAAAAAAAAAAAAAAAAAAA";

#[test]
fn keys_export_as_raw_and_as_firefox_ordered_jwk() {
    let s = page();
    assert_eq!(
        settle(
            &s,
            "hm(['verify','sign'],true).then(function(k){return S.exportKey('jwk',k)})\
             .then(JSON.stringify)"
        ),
        "{\"alg\":\"HS256\",\"ext\":true,\"k\":\"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\",\
         \"key_ops\":[\"sign\",\"verify\"],\"kty\":\"oct\"}"
    );
    assert_eq!(
        settle(
            &s,
            "S.importKey('raw',U(20),{name:'HMAC',hash:'SHA-1'},true,['sign']).then(function(k){\
             return S.exportKey('jwk',k)}).then(JSON.stringify)"
        ),
        "{\"alg\":\"HS1\",\"ext\":true,\"k\":\"AAAAAAAAAAAAAAAAAAAAAAAAAAA\",\
         \"key_ops\":[\"sign\"],\"kty\":\"oct\"}"
    );
    assert_eq!(
        settle(
            &s,
            "aes(['decrypt','encrypt'],true,16).then(function(k){return S.exportKey('jwk',k)})\
             .then(JSON.stringify)"
        ),
        "{\"alg\":\"A128GCM\",\"ext\":true,\"k\":\"AAAAAAAAAAAAAAAAAAAAAA\",\
         \"key_ops\":[\"encrypt\",\"decrypt\"],\"kty\":\"oct\"}"
    );
    // Raw export is a fresh ArrayBuffer of the key's bytes; base64url handles
    // every tail length and both of its own alphabet's specials.
    assert_eq!(
        settle(
            &s,
            "S.importKey('raw',unhex('fbff00fffe'),H256,true,['sign']).then(function(k){\
             return Promise.all([S.exportKey('raw',k),S.exportKey('jwk',k)])}).then(function(a){\
             return String(a[0])+hex(a[0])+' '+a[1].k})"
        ),
        "[object ArrayBuffer]fbff00fffe -_8A__4"
    );
    assert_eq!(
        settle(
            &s,
            "S.importKey('raw',unhex('fbff00ff'),H256,true,['sign']).then(function(k){\
             return S.exportKey('jwk',k)}).then(function(j){return j.k})"
        ),
        "-_8A_w"
    );
    // A non-extractable key exports as nothing; format is judged first.
    for (fmt, want) in [
        ("raw", ACCESS),
        ("jwk", ACCESS),
        ("spki", ACCESS),
        ("foo", SYNTAX),
    ] {
        let expr = format!("aes(['encrypt']).then(function(k){{return S.exportKey('{fmt}',k)}})");
        assert_eq!(settle(&s, &expr), want, "{fmt}");
    }
    for fmt in ["spki", "pkcs8"] {
        let expr =
            format!("aes(['encrypt'],true).then(function(k){{return S.exportKey('{fmt}',k)}})");
        assert_eq!(settle(&s, &expr), NOT_SUPPORTED, "{fmt}");
    }
    assert_eq!(
        settle(&s, "S.exportKey('raw', {})"),
        "TypeError: SubtleCrypto.exportKey: Argument 2 does not implement interface CryptoKey."
    );
}

#[test]
fn jwk_import_follows_the_measured_rules() {
    let s = page();
    let import = |jwk: &str, alg: &str, ext: bool, usages: &str| {
        settle(
            &s,
            &format!(
                "S.importKey('jwk',{jwk},{alg},{ext},{usages}).then(function(k){{\
                 return JSON.stringify(k.algorithm)+k.usages+k.extractable}})"
            ),
        )
    };
    let aes128 = "{\"name\":\"AES-GCM\",\"length\":128}encrypt";
    // Accepted: a bare key; a matching alg/use/ext/key_ops; a superset key_ops;
    // ext:false for a non-extractable import; any present kty at all.
    for jwk in [
        format!("{{kty:'oct',k:'{K16}'}}"),
        format!("{{kty:'oct',k:'{K16}',alg:'A128GCM',use:'enc',ext:true}}"),
        format!("{{kty:'oct',k:'{K16}',key_ops:['encrypt','decrypt']}}"),
        format!("{{kty:'RSA',k:'{K16}'}}"),
        "{kty:'oct',k:'----------------------'}".to_string(),
    ] {
        assert_eq!(
            import(&jwk, "'AES-GCM'", true, "['encrypt']"),
            format!("{aes128}true"),
            "{jwk}"
        );
    }
    assert_eq!(
        import(
            &format!("{{kty:'oct',k:'{K16}',ext:false}}"),
            "'AES-GCM'",
            false,
            "['encrypt']"
        ),
        format!("{aes128}false")
    );
    assert_eq!(
        import(
            &format!("{{kty:'oct',k:'{K16}',alg:'HS256',use:'sig'}}"),
            "H256",
            true,
            "['sign']"
        ),
        "{\"name\":\"HMAC\",\"hash\":{\"name\":\"SHA-256\"},\"length\":128}signtrue"
    );
    // Refused, every one a DataError.
    for jwk in [
        "{kty:'oct'}".to_string(),
        format!("{{k:'{K16}'}}"),
        format!("{{kty:'oct',k:'{K16}',alg:'A256GCM'}}"),
        "{kty:'oct',k:'!!!'}".to_string(),
        format!("{{kty:'oct',k:'{K16}=='}}"),
        "{kty:'oct',k:'AAAAA'}".to_string(),
        "{kty:'oct',k:'AAAAAAA'}".to_string(),
        "{kty:'oct',k:'++++++++++++++++++++++'}".to_string(),
        format!("{{kty:'oct',k:'{K16}',ext:false}}"),
        format!("{{kty:'oct',k:'{K16}',key_ops:['decrypt']}}"),
        format!("{{kty:'oct',k:'{K16}',key_ops:'encrypt'}}"),
        format!("{{kty:'oct',k:'{K16}',use:'sig'}}"),
    ] {
        assert_eq!(
            import(&jwk, "'AES-GCM'", true, "['encrypt']"),
            DATA,
            "{jwk}"
        );
    }
    for jwk in [
        format!("{{kty:'oct',k:'{K16}',alg:'HS512'}}"),
        format!("{{kty:'oct',k:'{K16}',use:'enc'}}"),
    ] {
        assert_eq!(import(&jwk, "H256", true, "['sign']"), DATA, "{jwk}");
    }
    // HMAC with no hash is a SyntaxError even when the JWK names one.
    assert_eq!(
        import(
            &format!("{{kty:'oct',k:'{K16}',alg:'HS256'}}"),
            "{name:'HMAC'}",
            true,
            "['sign']"
        ),
        SYNTAX
    );
}

#[test]
fn generated_keys_are_random_and_firefox_sized() {
    let s = page();
    let gen = |alg: &str, usages: &str| {
        settle(
            &s,
            &format!(
                "S.generateKey({alg},true,{usages}).then(function(k){{return S.exportKey('raw',k)\
                 .then(function(b){{return JSON.stringify(k.algorithm)+k.usages+' '+b.byteLength\
                 }})}})"
            ),
        )
    };
    assert_eq!(
        gen("{name:'AES-GCM',length:'256'}", "['decrypt','encrypt']"),
        "{\"name\":\"AES-GCM\",\"length\":256}encrypt,decrypt 32"
    );
    assert_eq!(
        gen("{name:'AES-GCM',length:192}", "['encrypt']"),
        "{\"name\":\"AES-GCM\",\"length\":192}encrypt 24"
    );
    // HMAC defaults to the hash's block size; an explicit length is kept as
    // given and backed by its whole bytes.
    for (hash, bits) in [
        ("SHA-1", 512),
        ("SHA-256", 512),
        ("SHA-384", 1024),
        ("SHA-512", 1024),
    ] {
        assert_eq!(
            gen(&format!("{{name:'HMAC',hash:'{hash}'}}"), "['sign']"),
            format!(
                "{{\"name\":\"HMAC\",\"hash\":{{\"name\":\"{hash}\"}},\"length\":{bits}}}sign {}",
                bits / 8
            )
        );
    }
    assert_eq!(
        gen("{name:'HMAC',hash:'SHA-256',length:100}", "['sign']"),
        "{\"name\":\"HMAC\",\"hash\":{\"name\":\"SHA-256\"},\"length\":100}sign 12"
    );
    // Real entropy: two generated keys differ, and a generated key works.
    assert_eq!(
        settle(
            &s,
            "Promise.all([1,2].map(function(){return S.generateKey({name:'AES-GCM',length:256},\
             true,['encrypt','decrypt']).then(function(k){return S.exportKey('raw',k).then(hex)\
             })})).then(function(a){return a[0]!==a[1]&&a[0].length===64})"
        ),
        "true"
    );
    assert_eq!(
        settle(
            &s,
            "S.generateKey({name:'AES-GCM',length:128},false,['encrypt','decrypt']).then(\
             function(k){return S.encrypt(GCM,k,new Uint8Array([7,8])).then(function(c){\
             return S.decrypt(GCM,k,c)}).then(hex)})"
        ),
        "0708"
    );
    for (alg, usages, want) in [
        ("{name:'AES-GCM'}", "['encrypt']", SYNTAX),
        ("'AES-GCM'", "['encrypt']", SYNTAX),
        ("{name:'AES-GCM',length:100}", "['encrypt']", OPERATION),
        ("{name:'AES-GCM',length:128}", "[]", SYNTAX),
        ("{name:'AES-GCM',length:128}", "['sign']", SYNTAX),
        ("{name:'HMAC'}", "['sign']", SYNTAX),
        ("'HMAC'", "['sign']", SYNTAX),
        ("{name:'HMAC',hash:'SHA-1'}", "['encrypt']", SYNTAX),
        ("{name:'HMAC',hash:'SHA-1'}", "[]", SYNTAX),
        ("{name:'HMAC',hash:'SHA-256',length:0}", "['sign']", DATA),
        ("{name:'FOO'}", "['sign']", NOT_SUPPORTED),
        ("'SHA-256'", "['sign']", NOT_SUPPORTED),
    ] {
        assert_eq!(gen(alg, usages), want, "{alg} {usages}");
    }
    assert_eq!(
        settle(
            &s,
            "S.generateKey({name:'AES-GCM',length:128},true,'encrypt')"
        ),
        "TypeError: SubtleCrypto.generateKey: Argument 3 can't be converted to a sequence."
    );
    assert_eq!(
        settle(&s, "S.generateKey({name:'AES-GCM',length:128},true)"),
        "TypeError: SubtleCrypto.generateKey: At least 3 arguments required, but only 2 passed"
    );
}
