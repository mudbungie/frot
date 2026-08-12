//! CSS string escapes — the one place `\` is understood.
//!
//! Two faces of the same rule, so no scanner has to reinvent it:
//!
//! - [`Quoting`] tells a *scanner* whether a character sits inside a string
//!   literal, so a `\"` cannot be mistaken for the closing delimiter. Every
//!   top-level splitter in the CSS engine ([`super::selparse::split_top`],
//!   [`super::parse`]'s name/value colon, [`super::content`]'s value
//!   tokenizer) rides it.
//! - [`unquote`] *decodes* a string token into the text it stands for: hex
//!   escapes (`\e609`), simple escaped code points (`\"`, `\\`), escaped-newline
//!   continuations, and the replacement-character edges.
//!
//! Decoding follows CSS Syntax Level 3 §4.3.7 (consume an escaped code point):
//! 1–6 hex digits optionally followed by one whitespace unit; a null, surrogate,
//! or out-of-range value becomes U+FFFD; a `\` before a newline is a line
//! continuation contributing nothing; a `\` at end of input contributes nothing.

/// Whether a character sits inside a CSS string literal, `\` escapes honoured.
///
/// Fed one character at a time in source order; [`Quoting::feed`] returns `true`
/// for every character of a string literal *including its delimiters*, so a
/// scanner can treat exactly the `false` characters as structural.
#[derive(Default)]
pub struct Quoting {
    quote: Option<char>,
    escaped: bool,
}

impl Quoting {
    /// Advance over `ch`, returning whether it is part of a string literal.
    pub fn feed(&mut self, ch: char) -> bool {
        match self.quote {
            Some(q) => {
                if self.escaped {
                    self.escaped = false;
                } else if ch == '\\' {
                    self.escaped = true;
                } else if ch == q {
                    self.quote = None;
                }
                true
            }
            None => {
                let opens = ch == '"' || ch == '\'';
                if opens {
                    self.quote = Some(ch);
                }
                opens
            }
        }
    }
}

/// Decode a string token — `tok` starts with its `"`/`'` delimiter — into the
/// text it denotes. Reads to the matching unescaped delimiter, or to the end of
/// the token when the literal is unterminated.
pub fn unquote(tok: &str) -> String {
    let b: Vec<char> = tok.chars().collect();
    let q = b[0];
    let mut out = String::new();
    let mut i = 1;
    while i < b.len() && b[i] != q {
        if b[i] == '\\' {
            i += 1;
            escaped(&b, &mut i, &mut out);
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// Consume the escape sequence whose `\` was just passed (`*i` is the character
/// after it) and append what it denotes — nothing, for a line continuation or a
/// trailing `\`.
fn escaped(b: &[char], i: &mut usize, out: &mut String) {
    let Some(&c) = b.get(*i) else { return };
    if matches!(c, '\n' | '\r' | '\u{c}') {
        *i += ws_len(b, *i);
    } else if c.is_ascii_hexdigit() {
        let (mut v, start) = (0u32, *i);
        while *i - start < 6 {
            let Some(d) = b.get(*i).and_then(|c| c.to_digit(16)) else {
                break;
            };
            v = v * 16 + d;
            *i += 1;
        }
        *i += ws_len(b, *i);
        out.push(match char::from_u32(v) {
            Some(c) if v != 0 => c,
            _ => '\u{fffd}',
        });
    } else {
        out.push(c);
        *i += 1;
    }
}

/// Length of the one whitespace unit at `i` — `\r\n` counts as one, any other
/// whitespace character as one, a non-whitespace character as none.
fn ws_len(b: &[char], i: usize) -> usize {
    match b.get(i) {
        Some('\r') if b.get(i + 1) == Some(&'\n') => 2,
        Some(c) if c.is_whitespace() => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests;
