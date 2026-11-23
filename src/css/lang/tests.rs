use crate::css::{query_all, UnsupportedSelector};
use crate::dom::{Document, NodeKind};

fn ids(html: &str, selector: &str) -> String {
    let doc = Document::parse(html);
    let hits = query_all(&doc, Document::ROOT, selector).unwrap();
    let id = |&n: &u32| match &doc.node(n).kind {
        NodeKind::Element(e) => e.attr("id").unwrap_or("-").to_string(),
        other => panic!("matched a non-element: {other:?}"),
    };
    let named = hits.iter().map(id).filter(|i| i != "-");
    named.collect::<Vec<_>>().join(" ")
}

// Measured in Chrome 158 (`bl-2a85`).
const LANGS: &str = "<p id=l0>x</p><div id=l1 lang=fr-CA><p id=l2>x</p><p id=l3 lang=''>x</p>\
    <p id=l4 lang=EN>x</p></div><div id=l5 lang=fr><span id=l6 xml:lang=es>x</span></div>\
    <svg id=sv><text id=l7 xml:lang=it>x</text></svg><div id=l8 lang=zh-Hant-TW></div>\
    <div id=l9 lang=de-Latn-DE></div><div id=l10 lang=i-klingon></div>";

#[test]
fn lang_matches_a_prefix_ending_at_a_hyphen() {
    assert_eq!(ids(LANGS, ":lang(fr)"), "l1 l2 l5 l6");
    assert_eq!(ids(LANGS, ":lang(FR-ca)"), "l1 l2");
    assert_eq!(ids(LANGS, ":lang(en)"), "l4");
    assert_eq!(ids(LANGS, ":lang(it)"), "l7");
    assert_eq!(
        ids(LANGS, ":lang(es), :lang(zh-tw), :lang(de-de), :lang(f)"),
        ""
    );
    assert_eq!(ids(LANGS, ":lang(i-klingon)"), "l10");
}

#[test]
fn without_lang_the_last_content_language_pragma_applies() {
    let page = |metas: &[&str]| {
        let metas: String = metas
            .iter()
            .map(|m| format!("<meta http-equiv=Content-Language content='{m}'>"))
            .collect();
        format!("<head>{metas}<meta http-equiv=refresh content=5><meta http-equiv=content-language></head><body id=b>")
    };
    assert_eq!(ids(&page(&["de", "it"]), "body:lang(it)"), "b");
    assert_eq!(ids(&page(&["de", "it"]), "body:lang(de)"), "");
    assert_eq!(
        ids(&page(&["de", "es,fr"]), "body:lang(es), body:lang(de)"),
        ""
    );
    assert_eq!(ids(&page(&["de-AT"]), "body:lang(de)"), "b");
    assert_eq!(ids(&page(&[]), "body:lang(de)"), "");
}

#[test]
fn lang_takes_one_identifier() {
    let doc = Document::parse("<p lang=fr>");
    for bad in [":lang()", ":lang(\"fr\")", ":lang(fr, en)", ":lang(fr en)"] {
        let got = query_all(&doc, Document::ROOT, bad);
        assert_eq!(got, Err(UnsupportedSelector), "{bad}");
    }
}

const DIRS: &str = "<div id=d1 dir=rtl><p id=d2>x</p><p id=d3 dir=LTR>x</p><p id=d4 dir=auto>abc</p>\
    <p id=d5 dir=auto>\u{5e9}</p><p id=d6 dir=bogus>x</p></div><bdi id=d7>\u{5e9}</bdi><bdi id=d8>abc</bdi>\
    <p id=d9 dir=auto></p><p id=d10 dir=auto><span>\u{661}</span>\u{5e9}</p>\
    <input id=d11 dir=auto value=\u{5e9}><textarea id=d12 dir=auto>\u{5e9}</textarea>\
    <div id=d14 dir=rtl><input id=d15 type=tel><input id=d16><input id=d17 type=tel dir=rtl></div>\
    <svg id=s1 dir=rtl><text id=s2>x</text><foreignObject><p id=s3 dir=rtl></p></foreignObject></svg>";

#[test]
fn dir_follows_the_attribute_and_inherits() {
    assert_eq!(
        ids(DIRS, ":dir(rtl)"),
        "d1 d2 d5 d6 d7 d10 d11 d12 d14 d16 d17 s3"
    );
    assert_eq!(ids(DIRS, "p:dir(LTR)"), "d3 d4 d9");
    assert_eq!(ids(DIRS, "svg:dir(ltr), svg *:dir(ltr)"), "s1 s2");
}

const AUTO: &str = "<div dir=rtl><p id=a1 dir=auto></p><p id=a2 dir=auto>123</p>\
    <p id=a3 dir=auto><span dir=ltr>abc</span>\u{5e9}</p><p id=a4 dir=auto><bdi>abc</bdi>\u{5e9}</p>\
    <p id=a5 dir=auto><script>var x='a'</script>\u{5e9}</p><p id=a6 dir=auto><span dir=bogus>abc</span></p>\
    <p id=a7 dir=auto><img alt=abc>\u{5e9}</p><p id=a8 dir=auto>!!! \u{5e9}</p>\
    <input id=a10 dir=auto><textarea id=a13 dir=auto></textarea><bdi id=a14></bdi>\
    <p id=a16 dir=auto><style>p{}</style>\u{5e9}</p><p id=a17 dir=auto><textarea>abc</textarea>\u{5e9}</p>\
    <p id=a18 dir=auto>\u{3a9}\u{5e9}</p><p id=a19 dir=auto><!-- c -->\u{10900}</p><p id=a20 dir=auto>\u{1e900}</p></div>";

#[test]
fn dir_auto_reads_the_first_strong_character() {
    assert_eq!(ids(AUTO, ":dir(rtl)"), "a3 a4 a5 a7 a8 a16 a17 a19 a20");
}

#[test]
fn dir_takes_rtl_or_ltr() {
    let doc = Document::parse("<p>");
    for bad in [":dir()", ":dir(rtl ltr)", ":dir(\"rtl\")"] {
        let got = query_all(&doc, Document::ROOT, bad);
        assert_eq!(got, Err(UnsupportedSelector), "{bad}");
    }
    // Any other identifier is valid and matches nothing.
    assert_eq!(ids("<p id=p dir=auto>", ":dir(auto)"), "");
    assert_eq!(ids("<p id=p dir=auto>", "p:not(:dir(auto))"), "p");
}
