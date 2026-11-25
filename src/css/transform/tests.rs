use super::*;
use crate::css::compute;
use crate::dom::Document;

/// `--out text` of `html` under its own `<style>`.
fn painted(html: &str) -> String {
    let doc = Document::parse(html);
    crate::views::text::text(&doc, Some(&compute(&doc)))
}

#[test]
fn the_three_cases_and_none_paint_as_chrome_does() {
    assert_eq!(
        painted("<p style='text-transform:uppercase'>straße ﬁne</p>"),
        "STRASSE FINE"
    );
    assert_eq!(
        painted("<p style='text-transform:lowercase'>ΟΔΟΣ Ab</p>"),
        "οδος ab"
    );
    assert_eq!(
        painted("<p style='text-transform:capitalize'>o'neil x.y 3rd a_b ǆa ßa</p>"),
        "O'neil X.Y 3rd A_b ǅa ßa"
    );
    assert_eq!(painted("<p style='text-transform:none'>Ab</p>"), "Ab");
}

#[test]
fn it_is_inherited_and_the_ua_resets_it_on_controls() {
    let html = "<style>div{text-transform:uppercase}</style>\
                <div><span>in</span> <button>btn</button> \
                <em style='text-transform:none'>no</em></div>";
    assert_eq!(painted(html), "IN btn no");
}

#[test]
fn capitalize_reads_the_line_across_elements() {
    let html = "<style>p{text-transform:capitalize}</style>\
                <p>x<span>yz</span> hel<b>lo</b> a<img>b c<i style=display:none>d</i>e \
                f<span style=display:inline-block>g</span></p>";
    assert_eq!(painted(html), "Xyz Hello AB Ce FG");
}

#[test]
fn generated_content_takes_its_own_transform_else_its_elements() {
    let html = "<style>p{text-transform:uppercase}p::before{content:'gen '}\
                p::after{content:' TAIL';text-transform:lowercase}</style><p>mid</p>";
    assert_eq!(painted(html), "GEN MID tail");
}

#[test]
fn turkic_lang_tailors_the_dotted_and_dotless_i() {
    let html = "<div lang=tr-TR><p style='text-transform:uppercase'>istanbul</p>\
                <p style='text-transform:lowercase'>IŞIK İ</p>\
                <p lang=en style='text-transform:uppercase'>i</p>\
                <p lang=az style='text-transform:capitalize'>il</p></div>";
    assert_eq!(painted(html), "İSTANBUL\nışık i\nI\nİl");
}

#[test]
fn values_parse_as_chrome_keeps_them() {
    use Case::*;
    let p = |v| parse(v, Upper, Lower);
    assert_eq!(p("inherit"), Some(Upper));
    assert_eq!(p("unset"), Some(Upper));
    assert_eq!(p("revert"), Some(Lower));
    assert_eq!(p("revert-layer"), Some(Lower));
    assert_eq!(p("initial"), Some(None));
    assert_eq!(p(" UPPERCASE "), Some(Upper));
    assert_eq!(p("capitalize full-width"), Some(Capitalize));
    assert_eq!(p("full-size-kana"), Some(None));
    assert_eq!(p("none"), Some(None));
    // Dropped: two cases, an unknown word, nothing at all.
    assert_eq!(p("uppercase lowercase"), Option::None);
    assert_eq!(p("bogus"), Option::None);
    assert_eq!(p(""), Option::None);
}

#[test]
fn an_invalid_declaration_loses_to_the_valid_one_it_follows() {
    let html = "<style>p{text-transform:uppercase;text-transform:bogus}</style><p>a</p>";
    assert_eq!(painted(html), "A");
}

#[test]
fn titlecase_covers_the_four_digraphs() {
    let t = TextTransform::default();
    let titled: String = ['\u{1c4}', '\u{1c9}', '\u{1cc}', '\u{1f3}']
        .map(|c| t.title(c))
        .iter()
        .collect();
    assert_eq!(titled, "\u{1c5}\u{1c8}\u{1cb}\u{1f2}");
}

#[test]
fn apply_advances_the_context_and_keeps_it_over_empty_text() {
    let t = TextTransform::default();
    let mut prev = ' ';
    assert_eq!(t.apply("ab", &mut prev), Option::None);
    assert_eq!(prev, 'b');
    assert_eq!(t.apply("", &mut prev), Option::None);
    assert_eq!(prev, 'b');
}

/// `math-auto` on a one-letter `<mi>` (`bl-9b22`, Chrome 158).
#[test]
fn a_one_letter_mi_is_mathematical_italic() {
    let mi = |s: &str| painted(&format!("<math><mi>{s}</mi></math>"));
    for (from, to) in [
        ("x", "𝑥"),
        ("X", "𝑋"),
        ("h", "ℎ"),
        ("ı", "𝚤"),
        ("ȷ", "𝚥"),
        ("Ω", "𝛺"),
        ("ϴ", "𝛳"),
        ("∇", "𝛻"),
        ("α", "𝛼"),
        ("∂", "𝜕"),
        ("ϖ", "𝜛"),
        ("é", "é"),
        ("\u{3a2}", "\u{3a2}"),
        ("sin", "sin"),
        ("<span>z</span>", "𝑧"),
    ] {
        assert_eq!(mi(from), to, "{from}");
    }
    assert_eq!(painted("<math><mi mathvariant=NORMAL>n</mi></math>"), "n");
    assert_eq!(
        painted("<span style='text-transform:math-auto'>q</span>"),
        "𝑞"
    );
}
