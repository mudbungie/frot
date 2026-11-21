//! Known-answer tests for the AES modes — published vectors, never a value the
//! implementation produced for itself. The same code was also compared, call for
//! call, with Firefox 153.0esr's own answers (identity.md §3.19); those values
//! are pinned where a page reaches them, in `js::tests::subtle`.

use super::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// NIST SP 800-38A's four-block example plaintext, shared by every mode there.
const PLAIN: &str = "6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51\
                     30c81c46a35ce411e5fbc1191a0a52eff69f2445df4f9b17ad2b417be66c3710";
/// …and its three example keys (128, 192, 256 bits).
const KEYS: [&str; 3] = [
    "2b7e151628aed2a6abf7158809cf4f3c",
    "8e73b0f7da0e6452c810f32b809079e562f8ead2522c6b7b",
    "603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4",
];

/// SP 800-38A F.2.1 / F.2.3 / F.2.5 (CBC-AES128/192/256.Encrypt). The standard's
/// example is unpadded, so the four blocks are the first four of WebCrypto's
/// PKCS#7 output, which adds a fifth.
#[test]
fn cbc_matches_sp_800_38a() {
    let iv = unhex("000102030405060708090a0b0c0d0e0f");
    let want = [
        "7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2\
         73bed6b8e3c1743b7116e69e222295163ff1caa1681fac09120eca307586e1a7",
        "4f021db243bc633d7178183a9fa071e8b4d9ada9ad7dedf4e5e738763f69145a\
         571b242012fb7ae07fa9baac3df102e008b0e27988598881d920a9e64f5615cd",
        "f58c4c04d6e5f1ba779eabfb5f7bfbd69cfc4e967edb808d679f777bc6702c7d\
         39f23369a9d9bacfa530e26304231461b2eb05e2c39be9fcda6c19078c6a9d1b",
    ];
    for (key, want) in KEYS.iter().zip(want) {
        let sealed = cbc_seal(&unhex(key), &iv, &unhex(PLAIN)).unwrap();
        assert_eq!(sealed.len(), 80);
        assert_eq!(hex(&sealed[..64]), want);
        assert_eq!(hex(&cbc_open(&unhex(key), &iv, &sealed).unwrap()), PLAIN);
    }
}

#[test]
fn cbc_pads_and_refuses_what_does_not_unpad() {
    let (key, iv) = ([7u8; 16], [1u8; 16]);
    // The empty message is one block of padding; a whole block gains another.
    assert_eq!(cbc_seal(&key, &iv, b"").unwrap().len(), 16);
    assert_eq!(cbc_seal(&key, &iv, &[2; 16]).unwrap().len(), 32);
    let sealed = cbc_seal(&key, &iv, b"twenty bytes of text").unwrap();
    assert_eq!(
        cbc_open(&key, &iv, &sealed).unwrap(),
        b"twenty bytes of text"
    );
    // Not whole blocks, nothing at all, or padding that is not PKCS#7.
    assert_eq!(cbc_open(&key, &iv, &sealed[..15]), None);
    assert_eq!(cbc_open(&key, &iv, b""), None);
    assert_eq!(cbc_open(&key, &iv, &[0; 16]), None);
    // An IV that is not one block; a key that is not 16/24/32 bytes.
    assert_eq!(cbc_seal(&key, &[0; 12], b"x"), None);
    assert_eq!(cbc_open(&key, &[0; 17], &sealed), None);
    assert_eq!(cbc_seal(&[0; 15], &iv, b"x"), None);
    assert_eq!(cbc_open(&[0; 33], &iv, &sealed), None);
}

/// SP 800-38A F.5.1 / F.5.3 / F.5.5 (CTR-AES128/192/256.Encrypt). The example
/// counter ends `…feff`, so its second block carries out of the low byte.
#[test]
fn ctr_matches_sp_800_38a() {
    let counter = unhex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let want = [
        "874d6191b620e3261bef6864990db6ce9806f66b7970fdff8617187bb9fffdff\
         5ae4df3edbd5d35e5b4f09020db03eab1e031dda2fbe03d1792170a0f3009cee",
        "1abc932417521ca24f2b0459fe7e6e0b090339ec0aa6faefd5ccc2c6f4ce8e94\
         1e36b26bd1ebc670d1bd1d665620abf74f78a7f6d29809585a97daec58c6b050",
        "601ec313775789a5b7a7f504bbf3d228f443e3ca4d62b59aca84e990cacaf5c5\
         2b0930daa23de94ce87017ba2d84988ddfc9c58db67aada613c2dd08457941a6",
    ];
    for (key, want) in KEYS.iter().zip(want) {
        let sealed = ctr(&unhex(key), &counter, 128, &unhex(PLAIN)).unwrap();
        assert_eq!(hex(&sealed), want);
        // Its own inverse; and a 16-bit counter carries the same way here.
        assert_eq!(
            hex(&ctr(&unhex(key), &counter, 128, &sealed).unwrap()),
            PLAIN
        );
        assert_eq!(
            ctr(&unhex(key), &counter, 16, &unhex(PLAIN)).unwrap(),
            sealed
        );
    }
}

/// A counter narrower than the block wraps within its own bits: the block after
/// `…feff` under an 8-bit counter is `…fe00`, not the standard's `…ff00`.
#[test]
fn ctr_wraps_within_the_counter_bits() {
    let key = unhex(KEYS[0]);
    let plain = unhex(PLAIN);
    let top = unhex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
    let wrapped = unhex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfe00");
    let narrow = ctr(&key, &top, 8, &plain).unwrap();
    assert_eq!(narrow[..16], ctr(&key, &top, 128, &plain).unwrap()[..16]);
    assert_eq!(
        narrow[16..],
        ctr(&key, &wrapped, 128, &plain[16..]).unwrap()
    );
    // A 1-bit counter alternates between two blocks, however long the message.
    let even = unhex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfefe");
    let two = ctr(&key, &even, 1, &[0; 67]).unwrap();
    assert_eq!(two[..32], two[32..64]);
    assert_eq!(two[..3], two[64..]);
    assert_eq!(two[16..32], ctr(&key, &top, 128, &[0; 16]).unwrap());
    // Nothing to encrypt is nothing.
    assert_eq!(ctr(&key, &top, 64, b"").unwrap(), b"");
    // No counter bits, more than a block of them, a short block, a bad key.
    assert_eq!(ctr(&key, &top, 0, &plain), None);
    assert_eq!(ctr(&key, &top, 129, &plain), None);
    assert_eq!(ctr(&key, &top[..15], 64, &plain), None);
    assert_eq!(ctr(&key[..5], &top, 64, &plain), None);
}

/// The McGrew–Viega GCM specification's test cases (the vectors NIST adopted):
/// 2 (AES-128), 8 (AES-192) and 14 (AES-256) encrypt one zero block under a zero
/// key and nonce.
#[test]
fn gcm_matches_the_specification_zero_vectors() {
    let zero_block = [0u8; 16];
    let cases = [
        (
            16,
            "0388dace60b6a392f328c2b971b2fe78ab6e47d42cec13bdf53a67b21257bddf",
        ),
        (
            24,
            "98e7247c07f0fe411c267e4384b0f6002ff58d80033927ab8ef4d4587514f0fb",
        ),
        (
            32,
            "cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919",
        ),
    ];
    for (len, want) in cases {
        let key = vec![0u8; len];
        let sealed = gcm_seal(&key, &[0; 12], &[], 16, &zero_block).unwrap();
        assert_eq!(hex(&sealed), want, "{len}-byte key");
        assert_eq!(
            gcm_open(&key, &[0; 12], &[], 16, &sealed).unwrap(),
            zero_block
        );
    }
}

/// Test cases 4, 5 and 6 of the same specification: one key, associated data
/// and 60-byte plaintext under a 96-bit, a 64-bit and a 480-bit nonce — the
/// last two take the GHASH route to the pre-counter block.
#[test]
fn gcm_matches_the_specification_for_every_nonce_length() {
    let key = unhex("feffe9928665731c6d6a8f9467308308");
    let aad = unhex("feedfacedeadbeeffeedfacedeadbeefabaddad2");
    let plain = unhex(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39",
    );
    let cases = [
        (
            "cafebabefacedbaddecaf888",
            "42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e\
             21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091\
             5bc94fbc3221a5db94fae95ae7121a47",
        ),
        (
            "cafebabefacedbad",
            "61353b4c2806934a777ff51fa22a4755699b2a714fcdc6f83766e5f97b6c7423\
             73806900e49f24b22b097544d4896b424989b5e1ebac0f07c23f4598\
             3612d2e79e3b0785561be14aaca2fccb",
        ),
        (
            "9313225df88406e555909c5aff5269aa6a7a9538534f7da1e4c303d2a318a728\
             c3c0c95156809539fcf0e2429a6b525416aedbf5a0de6a57a637b39b",
            "8ce24998625615b603a033aca13fb894be9112a5c3a211a8ba262a3cca7e2ca7\
             01e4a9a4fba43c90ccdcb281d48c7c6fd62875d2aca417034c34aee5\
             619cc5aefffe0bfa462af43c1699d050",
        ),
    ];
    for (iv, want) in cases {
        let iv = unhex(iv);
        let sealed = gcm_seal(&key, &iv, &aad, 16, &plain).unwrap();
        assert_eq!(hex(&sealed), want);
        assert_eq!(gcm_open(&key, &iv, &aad, 16, &sealed).unwrap(), plain);
        // A shorter tag is the leading bytes of the full one (SP 800-38D).
        for tag_len in [4, 8, 12, 13, 14, 15] {
            let short = gcm_seal(&key, &iv, &aad, tag_len, &plain).unwrap();
            assert_eq!(short, sealed[..60 + tag_len]);
            assert_eq!(gcm_open(&key, &iv, &aad, tag_len, &short).unwrap(), plain);
            // …and is not accepted as a tag of another length.
            assert_eq!(gcm_open(&key, &iv, &aad, tag_len - 1, &short), None);
        }
    }
}

#[test]
fn gcm_refuses_what_it_cannot_do_and_what_does_not_authenticate() {
    let key = [0u8; 32];
    let sealed = gcm_seal(&key, &[0; 12], b"aad", 16, b"secret").unwrap();
    // A flipped bit anywhere, other associated data, another key or nonce.
    for at in [0, sealed.len() - 1] {
        let mut flipped = sealed.clone();
        flipped[at] ^= 1;
        assert_eq!(gcm_open(&key, &[0; 12], b"aad", 16, &flipped), None);
    }
    assert_eq!(gcm_open(&key, &[0; 12], b"", 16, &sealed), None);
    assert_eq!(gcm_open(&[1; 32], &[0; 12], b"aad", 16, &sealed), None);
    assert_eq!(gcm_open(&key, &[1; 12], b"aad", 16, &sealed), None);
    assert_eq!(gcm_open(&key, &[0; 16], b"aad", 16, &sealed), None);
    // Shorter than a tag: nothing to authenticate.
    assert_eq!(gcm_open(&key, &[0; 12], b"aad", 16, &sealed[..4]), None);
    // A key that is not 16/24/32 bytes; a tag longer than GCM has.
    assert_eq!(gcm_seal(&[0; 5], &[0; 12], b"", 16, b"x"), None);
    assert_eq!(gcm_open(&[0; 5], &[0; 12], b"", 16, &sealed), None);
    assert_eq!(gcm_seal(&key, &[0; 12], b"", 17, b"x"), None);
    assert_eq!(gcm_open(&key, &[0; 12], b"", 17, &sealed), None);
    // The empty message seals to a bare tag and opens to nothing.
    let tag = gcm_seal(&key, &[0; 12], b"", 16, b"").unwrap();
    assert_eq!(tag.len(), 16);
    assert_eq!(gcm_open(&key, &[0; 12], b"", 16, &tag).unwrap(), b"");
}

/// RFC 3394 §4.1–4.3 (128 bits of key data under a 128-, 192- and 256-bit KEK)
/// and §4.6 (256 bits under a 256-bit KEK).
#[test]
fn kw_matches_rfc_3394() {
    let kek = unhex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
    let data = unhex("00112233445566778899aabbccddeeff000102030405060708090a0b0c0d0e0f");
    let cases = [
        (16, 16, "1fa68b0a8112b447aef34bd8fb5a7b829d3e862371d2cfe5"),
        (24, 16, "96778b25ae6ca435f92b5b97c050aed2468ab8a17ad84e5d"),
        (32, 16, "64e8c3f9ce0f5ba263e9777905818a2a93c8191e7d6e8ae7"),
        (
            32,
            32,
            "28c9f404c4b810f4cbccb35cfb87f8263f5786e2d80ed326cbc7f0e71a99f43bfb988b9b7a02dd21",
        ),
    ];
    for (kek_len, data_len, want) in cases {
        let wrapped = kw_wrap(&kek[..kek_len], &data[..data_len]).unwrap();
        assert_eq!(hex(&wrapped), want);
        assert_eq!(
            kw_unwrap(&kek[..kek_len], &wrapped).unwrap(),
            data[..data_len]
        );
    }
}

#[test]
fn kw_refuses_what_is_not_whole_words_and_what_fails_its_check() {
    let kek = [7u8; 16];
    let wrapped = kw_wrap(&kek, &[9; 32]).unwrap();
    let mut flipped = wrapped.clone();
    flipped[39] ^= 1;
    assert_eq!(kw_unwrap(&kek, &flipped), None);
    assert_eq!(kw_unwrap(&[8; 16], &wrapped), None);
    // Under two words to wrap, under three to unwrap, or not whole words.
    assert_eq!(kw_wrap(&kek, &[9; 8]), None);
    assert_eq!(kw_wrap(&kek, &[9; 20]), None);
    assert_eq!(kw_unwrap(&kek, &wrapped[..16]), None);
    assert_eq!(kw_unwrap(&kek, &wrapped[..33]), None);
    // A key that is not 16/24/32 bytes.
    assert_eq!(kw_wrap(&[7; 17], &[9; 16]), None);
    assert_eq!(kw_unwrap(&[7; 17], &wrapped), None);
}
