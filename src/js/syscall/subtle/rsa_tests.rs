//! RSA against the key and signatures of the Firefox 153.0esr capture
//! (`bl-2763`; `firefox153rsa.txt`): PKCS #1 v1.5 signatures are deterministic,
//! so Firefox's own must come out byte for byte.

use super::rsa;

fn h(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

const PK8: &str = "308204bd020100300d06092a864886f70d0101010500048204a7308204a30201000282010100a434d40bfde34fc3e7e9c31ead669751ff70614ff062707691490639d55c972c11b0930ed222cbe6dc6527dad878e1955c4ec1f06758f7c20aa1ab6a118d1813d85d6cace95768a71582095eab637f42cd9b2452c281300003fbcdda2712c3297862823716ab88e7b991cbfe3355204d76e64fc334ccab686fef3a1a9d72ace784d2fa85b0cc56673c4d7db665516f268212f585e4621ad24a11de6f1a5468697943f0b6323991502f23f2db9a5cc4f801363026812f0a78a01f22238a37a04bf3cab1cb611bc70accf100841d6e49d660611e187c292e00f194c8beebd46117344328989032e2fbbeeb18ce4e70641cda1ba69a74c14a40aea8fa95c978ab69020301000102820100039f480a2d033afae1a918c270257233148515c5c8a75de9ba520d0ec633c655ea31fb472415866a9a4954f4113b0c11850ec107c78fd6849e42c0b5949b8880c4e612aea1c5bed63e0ae1f55cfc7de32b06956a349b85993527f47b851f21d4f95b3044d228333096b9ae46a0e540d2d4a0f1f8d1625914485bd396a0b5d5e08b1e88e929a0c4ebb776b7523cb1daf2bea367626dc218a29e5f6f8c6220158e496eca34093719ccfbd64174dc7ad5ac78ca71b011c60a82eea018e1744e43f3f32d1d51de9af802a26f85b518a19a4e05d7b1d07d653071e7385437f1ac50609c6ab43cd5c42bfd8999d24b40dd8fbd322a889a9aa817924cdcecaaf066550102818100e20618cb3075f3607e853720bce408218527cc853af79ca5006323317ddc9f08be1a5864fe55b13beaf73cb50d80320b0800369ab1d7ef8bb50d2a69e1958ef6a1e6436cdeeb8429a56d3f7b3cff8e7d5f6f5611df5d0918fce3ae2eb6beef76864180fac2dae1a2e333b6c4d78313f688af26822ee9b790e3a79cc399f3cc8902818100b9fbebc1f61bbc214b5ed5249e5140310949b7a43344e1ce4fdee950524e67fa06b413e4394be0c5c3fc1492b2edc8598f54d1f7559b686ec0dc573136e0f1d571d0829e455c3ed15fcc732fc64722d448f2c959cdab6998dc278d61a77dcb6bc4da676aeb907a67646b14673aedea13723b5005c0515d41bfb0feee08ceefe10281803e4acbde84169de2c43bd6bbb1136fa23d5e80a15f650ac8f6d85bc4b21971c329117db727d95a255756986695b7f512d13badd0e89ec8a84a57bc8ebddb483174e14f47dc1030b6819f2055c8b3ee03639ffdaaa352b2c52e3d59ae0a33892fb344ae556e0a81113168f9403add274d54d260dc942d59a816f604e06bb7a3710281810089cc22806245685d4ec7a3aa6ef68a560aa8a8bcab13d52e143577ae805d0162d986a0da4ead6e8b78cb13bf217d561d2d3a4e1b303d65e1f1590fed31d8f6b50b8fcec74f62aa545ce35150cb5ab76e8be1fed75fb02a1c0cb9ed61b64458ecaf3c96b53c759e8b6e767bf6844df449963065f8ece61bef7404c2cd408f7a610281804c15f7edeb5d29113853ff575fa7aa2fb49903d812910b03808d28566a38aa679840e386351af79c791b4a1eb5d9f397205dd1fd87695fbbd3fd0f9e3b632751b179cbfbbda16b14231286543bf55f3835bd8e9eda04740f23274841dd5979c2b9d62015e7e8d4417746e9f2aac46a8b3e026d6ddefad6667e69804a97ff5454";
const SPKI: &str = "30820122300d06092a864886f70d01010105000382010f003082010a0282010100a434d40bfde34fc3e7e9c31ead669751ff70614ff062707691490639d55c972c11b0930ed222cbe6dc6527dad878e1955c4ec1f06758f7c20aa1ab6a118d1813d85d6cace95768a71582095eab637f42cd9b2452c281300003fbcdda2712c3297862823716ab88e7b991cbfe3355204d76e64fc334ccab686fef3a1a9d72ace784d2fa85b0cc56673c4d7db665516f268212f585e4621ad24a11de6f1a5468697943f0b6323991502f23f2db9a5cc4f801363026812f0a78a01f22238a37a04bf3cab1cb611bc70accf100841d6e49d660611e187c292e00f194c8beebd46117344328989032e2fbbeeb18ce4e70641cda1ba69a74c14a40aea8fa95c978ab690203010001";
const PK8_1024: &str = "30820277020100300d06092a864886f70d0101010500048202613082025d02010002818100cd48f1b05a9088387ca85c1ce33a33bab445d5be4ce0267fb55e4561222e098f0f44cc56905e046604afc0b97bc530d61d8438f57d5d28dfda72bad58b10c392221f44d1013013be616df95bd71aae465cc56f313f4dd1163eb63924fac59100955cdffe46422800929025c4ab7d7e1793353da356f17518a3512ed317a60a73020301000102818012b4d6f14f0f649a6121a6632b819e3d21384be5ef736d8eada44d86306b298bd3c45d2a76355b0940dc131fd3db803bc2accdd3f0fc9040f5d3069b76f4ac1d770ce7389bca02a8fb239bb9529bb414e1a39c912a955c16355fc603d80dfa2d8ec96d99add916e555cefcfa4fa6e8bbc4de78256e4299328abc0b0d80da8e81024100f2eb5488de96edb05e367a493e5543def44ea61237cdfddf04f177de473715960c7877cea0a5275626528edf46777570e1798db3854c5de001793d224740f8c3024100d856d209ef638bc71ea04fb53805160465def161f0125093956b1aa07617e71e04e7fdbe54bbd5b1efe9cb4f29a4cbaa65d94eb829177b7afe2741b30c100c91024100ed7a5c8115c24d09ff809899e4032d3fd86ccef3be7475fcc1f36230fe43fea95af5a6826d1027dd93c75fff2cff79e369ba106a7125101cfeb3ccad2725804b0241009cd0aa257b7a0beb04ff2d67edb87a122c09a3acdb495b50ef4118a6f19337fb61910c3437d583fc1c2cbb2ba1c30bc2735b469d6563d924f92f5987409a2d9102406572b5004a4e00b3a703f8070a87022c505c98452751885ce23deaf19f048d5e7fe829c9213345177ab064fe65e48304d75764dbcc7ba685c98ff54533836189";
/// Firefox's RSASSA-PKCS1-v1_5 / SHA-256 signature of 33 bytes of 4.
const SIG: &str = "61f5aa0f44fbe13706f7618ad63e6d6164a9345b8ae5e3dc4b788661e3acfc1256d41dac1ab71c9b157db2b29d31ecdd7ed5ef8f78ca01b4181cb3805d2e48e317646ca661742384b390d1df275e4d7c9eaa9b8f65bc3f1ceed0365e8f3a6d2373717612d74bed47cdd86580a8260224f04543a4e2794e31dd166c4a7554dbdb093ec4132534db088bcc75c51887ea19db5401b32b432fe3821c9fe307b14a224412e3a4dae76b3b763ad5f81a7cc5be33245d6544b8686fce5185e8ec498674f63f9213c867f3be9518d83cabcf03e2c5301d0cb7644a059825aa51e1531b764b593505191cee88ac06b6acdaabe4da920b5d482b2d7c7d945442fe84529a74";

#[test]
fn keys_the_provider_takes_and_refuses() {
    assert_eq!(rsa::check(true, &h(PK8)), 1);
    assert_eq!(rsa::check(false, &h(SPKI)), 1);
    assert_eq!(rsa::check(true, &h(PK8_1024)), -1);
    assert_eq!(rsa::check(true, &h(SPKI)), 0);
    assert_eq!(rsa::check(false, &[0x30, 0]), 0);
    for bits in [2048, 3072, 4096] {
        let [pk8, spki] = rsa::generate(bits).unwrap();
        assert_eq!((rsa::check(true, &pk8), rsa::check(false, &spki)), (1, 1));
    }
    assert_eq!(rsa::generate(8192), None);
}

#[test]
fn signatures_match_firefox_and_verify() {
    let data = [4; 33];
    let sig = rsa::sign(false, "SHA-256", &h(PK8), &data).unwrap();
    assert_eq!(sig, h(SIG));
    assert!(rsa::verify(false, "SHA-256", &h(SPKI), &sig, &data));
    assert!(!rsa::verify(false, "SHA-256", &h(SPKI), &sig, &[5; 33]));
    for (pss, hash) in [
        (false, "SHA-384"),
        (false, "SHA-512"),
        (true, "SHA-256"),
        (true, "SHA-384"),
        (true, "SHA-512"),
    ] {
        let sig = rsa::sign(pss, hash, &h(PK8), &data).unwrap();
        assert!(
            rsa::verify(pss, hash, &h(SPKI), &sig, &data),
            "{pss} {hash}"
        );
    }
    // SHA-1 verifies (legacy) but never signs; PSS has no SHA-1 at all; and
    // a key that is not one signs nothing.
    assert_eq!(rsa::sign(false, "SHA-1", &h(PK8), &data), None);
    assert!(!rsa::verify(false, "SHA-1", &h(SPKI), &sig, &data));
    assert_eq!(rsa::sign(true, "SHA-1", &h(PK8), &data), None);
    assert!(!rsa::verify(true, "SHA-1", &h(SPKI), &sig, &data));
    assert_eq!(rsa::sign(false, "SHA-256", &h(SPKI), &data), None);
}

#[test]
fn oaep_round_trips_under_every_hash_and_label() {
    let msg = [9; 16];
    for hash in ["SHA-1", "SHA-256", "SHA-384", "SHA-512"] {
        let sealed = rsa::crypt(false, hash, &h(SPKI), b"l", &msg).unwrap();
        assert_eq!(sealed.len(), 256);
        let opened = rsa::crypt(true, hash, &h(PK8), b"l", &sealed);
        assert_eq!(opened.as_deref(), Some(&msg[..]), "{hash}");
        assert_eq!(rsa::crypt(true, hash, &h(PK8), b"m", &sealed), None);
    }
    assert_eq!(rsa::crypt(false, "MD5", &h(SPKI), b"", &msg), None);
    assert_eq!(rsa::crypt(false, "SHA-256", &h(PK8), b"", &msg), None);
    assert_eq!(rsa::crypt(true, "SHA-256", &h(SPKI), b"", &msg), None);
    // 190 bytes fit a 2048-bit key under SHA-256; 191 do not.
    assert!(rsa::crypt(false, "SHA-256", &h(SPKI), b"", &[1; 190]).is_some());
    assert_eq!(rsa::crypt(false, "SHA-256", &h(SPKI), b"", &[1; 191]), None);
}
