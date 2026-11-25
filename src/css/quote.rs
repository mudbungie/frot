//! Quote marks: the `quotes` property, whether a `::before` / `::after`
//! inserts one, and the marks a language uses (`bl-73ee`). frot's `text`
//! paints no quote mark — the fidelity recorder reads only strings — so
//! accessible names are the one reader (`ax/name/contents/native.rs`).
//!
//! Measured in Chrome 158 (`Accessibility.getFullAXTree`, the name of
//! `<h2>a<q>t</q>c</h2>`):
//!
//! - The UA's `q::before { content: open-quote }` and `q::after { content:
//!   close-quote }` lose to any author `content`: `none`, `normal` or `""`
//!   on `::before` drops the open mark and with it the close one (a close
//!   quote at depth zero says nothing), on `::after` the close mark alone; a
//!   `content` holding `open-quote` / `close-quote` keeps it; a pseudo-element
//!   with `display: none` inserts nothing.
//! - `quotes` is inherited: `none` gives no marks, a list of strings gives
//!   its pairs — the first at the outer level, the last pair repeated deeper —
//!   and `auto` (the initial value) the language's.
//! - The language's marks are ICU's for the element's language as Chrome
//!   ships it ([`LANGS`]): the tag with `-` as `_`, tried whole and then with
//!   its last subtag dropped, case-insensitive; a language with none uses
//!   “ ” and ‘ ’. `es`, `pt`, `zh`, `zh-TW` and `en-GB` have none in Chrome.

use super::computed::Applied;
use super::selector::Pseudo;

/// Computed `quotes`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Quotes {
    #[default]
    Auto,
    /// The marks in pairs, open then close; `none` is the empty list.
    Strings(Vec<String>),
}

impl Quotes {
    pub(super) fn of(applied: &[Applied], parent: &Quotes) -> Quotes {
        let winner = applied
            .iter()
            .filter(|a| a.pseudo.is_none() && a.name == "quotes")
            .max_by_key(|a| a.key());
        let Some(value) = winner.map(|a| a.value.trim()) else {
            return parent.clone();
        };
        match value.to_ascii_lowercase().as_str() {
            "inherit" | "unset" => parent.clone(),
            "none" => Quotes::Strings(Vec::new()),
            "auto" | "initial" => Quotes::Auto,
            _ => strings(value).map_or_else(|| parent.clone(), Quotes::Strings),
        }
    }

    /// The open and close marks at nesting `depth` (0 outermost) for an
    /// element in language `lang`.
    pub fn pair(&self, depth: usize, lang: Option<&str>) -> (String, String) {
        match self {
            Quotes::Strings(v) if v.len() < 2 => (String::new(), String::new()),
            Quotes::Strings(v) => {
                let at = 2 * depth.min(v.len() / 2 - 1);
                (v[at].clone(), v[at + 1].clone())
            }
            Quotes::Auto => {
                let m = lang.and_then(marks).unwrap_or(['“', '”', '‘', '’']);
                let at = 2 * depth.min(1);
                (m[at].to_string(), m[at + 1].to_string())
            }
        }
    }
}

/// A `quotes` list of strings, two by two; `None` when it is not one.
fn strings(value: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut rest = value.trim_start();
    while let Some(q) = rest.chars().next().filter(|c| matches!(c, '"' | '\'')) {
        let end = rest[1..].find(q)? + 1;
        out.push(rest[1..end].to_string());
        rest = rest[end + 1..].trim_start();
    }
    (rest.is_empty() && !out.is_empty() && out.len() % 2 == 0).then_some(out)
}

/// Whether the `pseudo` box of an element inserts a quote mark: the author's
/// `content`, if one won, says so by naming one; else the UA's does for a
/// `<q>`. `shown` is false when the pseudo-element is `display: none`.
pub(super) fn inserts(applied: &[Applied], pseudo: Pseudo, q: bool, shown: bool) -> bool {
    let winner = applied
        .iter()
        .filter(|a| a.pseudo == Some(pseudo) && a.name == "content")
        .max_by_key(|a| a.key());
    shown
        && winner.map_or(q, |a| {
            let v = a.value.to_ascii_lowercase();
            v.contains("open-quote") || v.contains("close-quote")
        })
}

/// The languages Chrome 158 has marks for: open, close, inner open, inner
/// close. Measured over every two-letter language code and 120 more tags;
/// any other is the default.
const LANGS: &[(&str, [char; 4])] = &[
    ("am", ['«', '»', '‹', '›']),
    ("ar", ['”', '“', '’', '‘']),
    ("az_cyrl", ['«', '»', '‹', '›']),
    ("bg", ['„', '“', '„', '“']),
    ("bs_cyrl", ['„', '“', '‚', '‘']),
    ("ca", ['«', '»', '“', '”']),
    ("cs", ['„', '“', '‚', '‘']),
    ("de", ['„', '“', '‚', '‘']),
    ("el", ['«', '»', '“', '”']),
    ("et", ['„', '“', '‚', '‘']),
    ("fa", ['«', '»', '‹', '›']),
    ("fi", ['”', '”', '’', '’']),
    ("fr", ['«', '»', '«', '»']),
    ("fr_ca", ['«', '»', '”', '“']),
    ("fr_ch", ['«', '»', '‹', '›']),
    ("he", ['”', '”', '’', '’']),
    ("hr", ['„', '“', '‚', '‘']),
    ("hu", ['„', '”', '»', '«']),
    ("it", ['«', '»', '“', '”']),
    ("ja", ['「', '」', '『', '』']),
    ("lt", ['„', '“', '„', '“']),
    ("nb", ['«', '»', '‘', '’']),
    ("nl", ['‘', '’', '‘', '’']),
    ("nn", ['«', '»', '‘', '’']),
    ("no", ['«', '»', '‘', '’']),
    ("pl", ['„', '”', '«', '»']),
    ("pt_pt", ['«', '»', '“', '”']),
    ("ro", ['„', '”', '«', '»']),
    ("ru", ['«', '»', '„', '“']),
    ("sk", ['„', '“', '‚', '‘']),
    ("sl", ['„', '“', '‚', '‘']),
    ("sr", ['„', '”', '’', '’']),
    ("sv", ['”', '”', '’', '’']),
    ("uk", ['«', '»', '„', '“']),
    ("ur", ['”', '“', '’', '‘']),
    ("zh_hant", ['「', '」', '『', '』']),
];

/// The marks for `lang` ([module docs](self)).
fn marks(lang: &str) -> Option<[char; 4]> {
    let mut tag = lang.replace('-', "_").to_ascii_lowercase();
    loop {
        if let Some((_, m)) = LANGS.iter().find(|(l, _)| *l == tag) {
            return Some(*m);
        }
        tag.truncate(tag.rfind('_')?);
    }
}

#[cfg(test)]
mod tests {
    use super::Quotes;
    use crate::css::compute;
    use crate::dom::Document;

    fn of(html: &str) -> Quotes {
        let doc = Document::parse(html);
        compute(&doc).quotes(doc.find_by_tag("q")[0]).clone()
    }

    #[test]
    fn quotes_is_inherited_and_parsed() {
        let s = |v: &[&str]| Quotes::Strings(v.iter().map(|s| s.to_string()).collect());
        for (style, want) in [
            ("", Quotes::Auto),
            ("quotes:none", s(&[])),
            ("quotes:'<' '>' '[' ']'", s(&["<", ">", "[", "]"])),
            ("quotes:'<' '>' '['", s(&["(", ")"])),
            ("quotes:'<' '>' x", s(&["(", ")"])),
            ("quotes:'<", s(&["(", ")"])),
            ("quotes:inherit", s(&["(", ")"])),
            ("quotes:unset", s(&["(", ")"])),
            ("quotes:initial", Quotes::Auto),
            ("quotes:AUTO", Quotes::Auto),
        ] {
            let html = format!("<p style=\"quotes:'(' ')'\"><q style=\"{style}\">x</q></p>");
            let want = if style.is_empty() {
                s(&["(", ")"])
            } else {
                want
            };
            assert_eq!(of(&html), want, "{style}");
        }
        assert_eq!(of("<q>x</q>"), Quotes::Auto);
    }

    #[test]
    fn a_pair_by_depth_and_language() {
        let pair = |q: &Quotes, d, l| q.pair(d, l);
        let two = Quotes::Strings(["<", ">", "[", "]"].map(String::from).to_vec());
        assert_eq!(pair(&two, 5, None), ("[".into(), "]".into()));
        assert_eq!(
            pair(&Quotes::Strings(Vec::new()), 0, None),
            (String::new(), String::new())
        );
        let auto = Quotes::Auto;
        assert_eq!(pair(&auto, 0, None), ("“".into(), "”".into()));
        assert_eq!(pair(&auto, 3, Some("fr-CH")), ("‹".into(), "›".into()));
        assert_eq!(
            pair(&auto, 0, Some("zh-Hant-TW")),
            ("「".into(), "」".into())
        );
        assert_eq!(pair(&auto, 0, Some("zh-TW")), ("“".into(), "”".into()));
    }
}
