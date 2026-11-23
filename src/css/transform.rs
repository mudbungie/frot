//! `text-transform` (`bl-e48d`): the case a run of text is painted in, which
//! is the case Chrome's `innerText` and its accessible names carry. Inherited;
//! the UA sheet resets it to `none` on `<input>`, `<textarea>`, `<select>` and
//! `<button>`. Only rendered text is transformed — never an attribute (`alt`,
//! `title`, `aria-label`, a control's `value`). Measured in Chrome 158 on the
//! `transform` fixture.
//!
//! Capitalization follows Chrome rather than a word-break table: a letter is
//! capitalized unless the character before it *in the line* — across element
//! boundaries, so `x<span>yz</span>` keeps `yz` — is a letter, a digit, an
//! apostrophe, an underscore or a middle dot. `o'neil` is `O'neil`, `x.y` is
//! `X.Y`, `3rd` is `3rd`. A block, an atomic inline or a replaced element
//! starts a fresh line context. A capitalized letter takes its titlecase form
//! when that is a single character (`ǆ` → `ǅ`, `ß` stays `ß`).
//!
//! The `lang` that tailors casing is honoured for Turkish and Azeri alone
//! (`istanbul` → `İSTANBUL`, `IŞIK` → `ışık`). `full-width` and
//! `full-size-kana` are valid and change nothing Chrome reports.

use crate::dom::Element;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Case {
    #[default]
    None,
    Upper,
    Lower,
    Capitalize,
}

/// The computed `text-transform`, with the one piece of `lang` casing needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextTransform {
    pub case: Case,
    pub turkic: bool,
}

/// The tags the UA sheet gives `text-transform: none`.
const UA_NONE: &[&str] = &["input", "textarea", "select", "button"];

impl TextTransform {
    /// The element's computed value: `winner` is its declarations' values,
    /// most important first, and the first valid one decides; with none, the
    /// UA's, else the parent's.
    pub fn of<'a>(
        winners: impl Iterator<Item = &'a str>,
        el: &Element,
        parent: TextTransform,
    ) -> TextTransform {
        let ua = if UA_NONE.contains(&el.name.as_str()) {
            Case::None
        } else {
            parent.case
        };
        let turkic = el.attr("lang").map_or(parent.turkic, |l| {
            let primary = l.split(['-', '_']).next().unwrap_or_default();
            ["tr", "az"].iter().any(|t| primary.eq_ignore_ascii_case(t))
        });
        let mut winners = winners;
        let case = winners
            .find_map(|v| parse(v, parent.case, ua))
            .unwrap_or(ua);
        TextTransform { case, turkic }
    }

    /// A pseudo-element's value: its own declarations, else its element's.
    pub fn pseudo<'a>(self, winners: impl Iterator<Item = &'a str>) -> TextTransform {
        let mut winners = winners;
        let case = winners
            .find_map(|v| parse(v, self.case, self.case))
            .unwrap_or(self.case);
        TextTransform { case, ..self }
    }

    /// `text` as painted, `None` when the transform leaves it alone. `prev` is
    /// the character before it in the line (a space at a line's start); it is
    /// advanced past `text` whatever the case.
    pub fn apply(self, text: &str, prev: &mut char) -> Option<String> {
        let out = match self.case {
            Case::None => None,
            Case::Upper => Some(text.chars().map(|c| self.upper(c)).collect::<String>()),
            Case::Lower => Some(self.lower(text)),
            Case::Capitalize => Some(self.capitalize(text, *prev)),
        };
        *prev = text.chars().last().unwrap_or(*prev);
        out
    }

    fn upper(self, c: char) -> String {
        match c {
            'i' if self.turkic => "\u{130}".into(),
            c => c.to_uppercase().collect(),
        }
    }

    fn lower(self, text: &str) -> String {
        let text = match self.turkic {
            true => text.replace('I', "\u{131}").replace('\u{130}', "i"),
            false => text.to_owned(),
        };
        text.to_lowercase()
    }

    fn capitalize(self, text: &str, mut prev: char) -> String {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            match in_word(prev) {
                true => out.push(c),
                false => out.push(self.title(c)),
            }
            prev = c;
        }
        out
    }

    /// The titlecase of `c`: the four Latin digraphs have a form of their
    /// own; otherwise the uppercase, when it is one character.
    fn title(self, c: char) -> char {
        match c {
            '\u{1c4}'..='\u{1c6}' => '\u{1c5}',
            '\u{1c7}'..='\u{1c9}' => '\u{1c8}',
            '\u{1ca}'..='\u{1cc}' => '\u{1cb}',
            '\u{1f1}'..='\u{1f3}' => '\u{1f2}',
            c => {
                let up = self.upper(c);
                let mut chars = up.chars();
                match (chars.next(), chars.next()) {
                    (Some(u), None) => u,
                    _ => c,
                }
            }
        }
    }
}

/// Whether a letter after `c` continues `c`'s word (module docs).
fn in_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '\'' | '\u{2019}' | '_' | '\u{b7}' | '\u{2027}')
}

/// One `text-transform` value: `None` when Chrome would drop the declaration.
/// `inherit` and `unset` take the parent's case, `revert` the UA's.
fn parse(value: &str, parent: Case, ua: Case) -> Option<Case> {
    let value = value.trim().to_ascii_lowercase();
    match value.as_str() {
        "inherit" | "unset" => return Some(parent),
        "revert" | "revert-layer" => return Some(ua),
        "initial" => return Some(Case::None),
        _ => {}
    }
    let (mut case, mut seen) = (None, false);
    for token in value.split_ascii_whitespace() {
        seen = true;
        let next = match token {
            "none" => Case::None,
            "uppercase" => Case::Upper,
            "lowercase" => Case::Lower,
            "capitalize" => Case::Capitalize,
            "full-width" | "full-size-kana" => continue,
            _ => return None,
        };
        if case.replace(next).is_some() {
            return None;
        }
    }
    seen.then(|| case.unwrap_or_default())
}

#[cfg(test)]
mod tests;
