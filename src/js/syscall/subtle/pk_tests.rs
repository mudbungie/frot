//! The elliptic-curve primitives against published vectors and signatures
//! Firefox 153.0esr made (`bl-0cae`; the same ones `firefox153pk.txt` holds):
//! RFC 7518 Appendix C (ECDH), RFC 7748 §6.1 (X25519), RFC 8032 §7.1 test 1
//! (Ed25519), and ECDSA under every curve/hash pairing the provider cannot
//! sign natively.

use super::pk;

fn h(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

const P256_D: &str = "8e9b109e719098bf980487df1f5d77e9c2cdb55cfe9d1c355cde319f318e1960";
const P256: &str = "040600c73459380da296b6426c895a5319e49a1a82b58d9f3751df8152e8d648e9c598d77f7a811c06627bce7b2534d540268c95ec1933a640b9ea4339411426af";
const P384: &str = "0432a8a9616ecf66b332cf0ef87ede1914e5ba22a5efd897893805dc10e753b0a53dbf97b092541bee58613d90d3d9a896f426b685a1cabcd9eca0c340554602672e04b425174e5573648e6e42f397db1378a76d8828e94faced38b6b8346af2b6";
const P521: &str = "040072992cb3ac08ecf3e5c63dedec0d51a8c1f79ef2f82f94f3c737bf5de7986671eac625fe8257bbd0394644caaa3aaf8f27a4585fbbcad0f2457620085e5c8f42ad01dca6947bce88bc5790485ac97427342bc35f887d86d65a089377e247e60baa55e4e8501e2ada5724ac51d6909008033ebc10ac999b9d7f5cc2519f3fe1ea1d9475";
const ALICE_D: &str = "d3f3716913d4310a0026de741b3f18893afc8114f0c84682ba677e313a13988a";
const ALICE: &str = "04808d060082c176eed3e776a4ac598cc8672c1779f974eecc9b03411ca5b9495d48b5bfc527dfce53d6ac7115237d031ccff87a0570b77350a9e503ee7305a69b";
const BOB_D: &str = "5449836690d75caf29f0dd029ddb31b3ddb8aba9d2d515c5012465e817d4a9dc";
const BOB: &str = "04c1e349cb61ec70248ce801034c3834e1b88ebe1161cb25af38741f785fcfc4c47bc96708ef80952b53f8d2555fe72b841ed04588628b1d378a594939500ec9c9";
const XA: &str = "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a";
const XA_PUB: &str = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a";
const XB_PUB: &str = "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f";
const ED_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const ED_PUB: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

#[test]
fn agreement_matches_the_published_secrets() {
    let ecdh = "9e56d91d817135d372834283bf84269cfb316ea3da806a48f6daa7798cfe90c4";
    assert_eq!(pk::agree("P-256", &h(ALICE_D), &h(BOB)), Some(h(ecdh)));
    assert_eq!(pk::agree("P-256", &h(BOB_D), &h(ALICE)), Some(h(ecdh)));
    let x = "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742";
    assert_eq!(pk::agree("X25519", &h(XA), &h(XB_PUB)), Some(h(x)));
    // A point of another curve, a short scalar, an unknown curve: no secret.
    assert_eq!(pk::agree("P-256", &h(ALICE_D), &h(P384)), None);
    assert_eq!(pk::agree("P-256", &[1; 5], &h(BOB)), None);
    assert_eq!(pk::agree("P-192", &h(ALICE_D), &h(BOB)), None);
}

#[test]
fn a_public_key_is_computed_and_checked() {
    assert_eq!(pk::public("P-256", &h(P256_D)), Some(h(P256)));
    assert_eq!(pk::public("X25519", &h(XA)), Some(h(XA_PUB)));
    assert_eq!(pk::public("Ed25519", &h(ED_SEED)), Some(h(ED_PUB)));
    assert_eq!(pk::public("Ed25519", &[1; 31]), None);
    assert_eq!(pk::public("P-256", &[1; 31]), None);
    assert_eq!(pk::public("P-999", &h(P256_D)), None);
    for (curve, point, ok) in [
        ("P-256", P256, true),
        ("P-384", P384, true),
        ("P-521", P521, true),
        ("P-256", P384, false),
        ("X25519", XA_PUB, true),
        ("Ed25519", ED_PUB, true),
        ("P-999", P256, false),
    ] {
        assert_eq!(pk::point(curve, &h(point)).is_some(), ok, "{curve}");
    }
    // A point off the curve, a short Ed25519 key, and X25519's small-order
    // points (0 and 1), which agree on nothing.
    let mut off = h(P256);
    off[64] ^= 1;
    assert!(pk::point("P-256", &off).is_none());
    assert!(pk::point("Ed25519", &[0; 31]).is_none());
    assert!(pk::point("X25519", &[0; 32]).is_none());
    let mut one = [0; 32];
    one[0] = 1;
    assert!(pk::point("X25519", &one).is_none());
}

#[test]
fn a_generated_pair_is_a_working_pair() {
    for (curve, d, p) in [
        ("P-256", 32, 65),
        ("P-384", 48, 97),
        ("P-521", 66, 133),
        ("X25519", 32, 32),
        ("Ed25519", 32, 32),
    ] {
        let [private, public] = pk::generate(curve).unwrap();
        assert_eq!((private.len(), public.len()), (d, p), "{curve}");
        assert_eq!(pk::public(curve, &private), Some(public), "{curve}");
    }
    assert_eq!(pk::generate("P-999"), None);
}

/// Firefox's signatures over ten bytes of 5, under hashes longer and shorter
/// than the curve's own — the cut and the zero-fill both verify.
#[test]
fn ecdsa_reads_any_hash_with_any_curve() {
    let data = [5; 10];
    for (curve, point, hash, sig) in [
        ("P-256", P256, "SHA-1", "7d8efde3b079dc60dce87dff8302016041d47880b7fb0dbf5552cd9d95a312e4bc5deea4fd73cd69a745e6b4194d7c10ae0389b68bb432df56041268b9950932"),
        ("P-384", P384, "SHA-512", "ff976463660d0dbe1c623412d126c00158e98afb2f1150b4c5e64375582416f7b617066a643b9d493a443ba860349c64083ec175fc4e9e49502ee60e77d6599505e955b4c1b0535b15fc01253a852c716d03a7a0da5467031bda6c85b2c47f27"),
        ("P-521", P521, "SHA-1", "0024f635af5b00c9000aa74b23dc08d743a447ac5bd7fedce8906ec2c3b4ff75e038fe4374ab6c24dfcb64078d85a50838183e1b6e834271210dc791bb42dc2c63a601e2553527ce02eb537dc1a998fee89d5905747b68788262340f0cca637754825bd920f09800bca8deeb53b511edecd5ad882d0fca05845254e4328c4e3a9b1d00ae"),
    ] {
        assert!(pk::ecdsa_verify(curve, hash, &h(point), &h(sig), &data), "{curve} {hash}");
        assert!(!pk::ecdsa_verify(curve, hash, &h(point), &h(sig), &[6; 10]));
        assert!(!pk::ecdsa_verify(curve, "MD5", &h(point), &h(sig), &data));
    }
    assert!(!pk::ecdsa_verify(
        "X25519",
        "SHA-1",
        &h(XA_PUB),
        &[0; 64],
        &data
    ));
    for hash in ["SHA-1", "SHA-256", "SHA-384", "SHA-512"] {
        let sig = pk::ecdsa_sign("P-256", hash, &h(P256_D), &data).unwrap();
        assert_eq!(sig.len(), 64);
        assert!(
            pk::ecdsa_verify("P-256", hash, &h(P256), &sig, &data),
            "{hash}"
        );
    }
    // Signing is by the scalar alone; none of these can sign.
    assert_eq!(pk::ecdsa_sign("P-256", "MD5", &h(P256_D), &data), None);
    assert_eq!(pk::ecdsa_sign("P-256", "SHA-1", &[0; 32], &data), None);
    assert_eq!(pk::ecdsa_sign("P-256", "SHA-1", &[7; 200], &data), None);
    assert_eq!(pk::ecdsa_sign("X25519", "SHA-1", &h(XA), &data), None);
    assert_eq!(pk::ecdsa_sign("P-999", "SHA-1", &h(XA), &data), None);
}

#[test]
fn ed25519_signs_as_rfc_8032_does() {
    let sig = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
    assert_eq!(pk::ed25519_sign(&h(ED_SEED), &[]), Some(h(sig)));
    assert!(pk::ed25519_verify(&h(ED_PUB), &h(sig), &[]));
    assert!(!pk::ed25519_verify(&h(ED_PUB), &h(sig), &[1]));
    assert_eq!(pk::ed25519_sign(&[1; 31], &[]), None);
}

/// A compressed point comes back whole, either root by its prefix; one whose x
/// is not on the curve, or not below p, is no point.
#[test]
fn a_compressed_point_is_made_whole() {
    for (curve, whole) in [
        ("P-256", P256),
        ("P-384", P384),
        ("P-521", P521),
        ("P-256", ALICE),
        ("P-256", BOB),
    ] {
        let full = h(whole);
        let n = (full.len() - 1) / 2;
        let mut short = vec![2 + (full[full.len() - 1] & 1)];
        short.extend_from_slice(&full[1..=n]);
        assert_eq!(pk::point(curve, &short), Some(full.clone()), "{curve}");
        // The other prefix is the other root: same x, y negated.
        short[0] ^= 1;
        let other = pk::point(curve, &short).unwrap();
        assert_eq!(other[..=n], full[..=n]);
        assert_ne!(other, full);
    }
    let mut none = vec![2];
    none.extend_from_slice(&[0xff; 32]);
    assert_eq!(pk::point("P-256", &none), None);
    none[1..].copy_from_slice(&[0; 32]);
    none[32] = 1; // x = 1 has no y on P-256
    assert_eq!(pk::point("P-256", &none), None);
    // The compressed length with another prefix is not a compressed point.
    none[0] = 4;
    assert_eq!(pk::point("P-256", &none), None);
}
