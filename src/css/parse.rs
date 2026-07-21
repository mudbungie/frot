//! Stylesheet and declaration-list parsing.
//!
//! A best-effort CSS subset: comments are stripped, and malformed rules or
//! declarations are dropped rather than aborting the sheet. `@media` preludes
//! are evaluated against the fixed viewport ([`super::media`]) — a matching
//! block's rules join the cascade, a non-matching one contributes nothing.
//! Every other `@`-rule (`@supports`, `@layer`, `@import`, …) is still
//! skipped wholesale, including any `{…}` body.

use super::media;
use super::selector::Selector;
use super::selparse::{parse_selector_list, split_top};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decl {
    pub name: String,
    pub value: String,
    pub important: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    pub fn parse(src: &str) -> Stylesheet {
        let src = strip_comments(src);
        let mut rules = Vec::new();
        let chars: Vec<char> = src.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            i = skip_ws(&chars, i);
            if i >= chars.len() {
                break;
            }
            if chars[i] == '@' {
                let (name, prelude, body, next) = read_at_rule(&chars, i);
                i = next;
                // Only `@media` is evaluated; a body-less one (statement form)
                // parses as the empty sheet. Nested `@media` multiply through
                // the recursion. Other conditional groups are still skipped.
                if name == "media" && media::matches(&prelude) {
                    rules.extend(Stylesheet::parse(&body.unwrap_or_default()).rules);
                }
                continue;
            }
            let (prelude, body, next) = match read_rule(&chars, i) {
                Some(r) => r,
                None => break,
            };
            i = next;
            let selectors = parse_selector_list(&prelude);
            if selectors.is_empty() {
                continue;
            }
            let decls = parse_decls(&body);
            if decls.is_empty() {
                continue;
            }
            rules.push(Rule { selectors, decls });
        }
        Stylesheet { rules }
    }
}

/// Parse a declaration list (an inline `style=` attribute, or a rule body).
pub fn parse_decls(s: &str) -> Vec<Decl> {
    split_top(s, ';')
        .into_iter()
        .filter_map(|chunk| parse_decl(chunk.trim()))
        .collect()
}

fn parse_decl(s: &str) -> Option<Decl> {
    if s.is_empty() {
        return None;
    }
    let colon = top_colon(s)?;
    let name = s[..colon].trim().to_ascii_lowercase();
    let mut value = s[colon + 1..].trim().to_string();
    if name.is_empty() || value.is_empty() {
        return None;
    }
    let important = strip_important(&mut value);
    if value.is_empty() {
        return None;
    }
    Some(Decl {
        name,
        value,
        important,
    })
}

/// Index of the first `:` outside quotes — the name/value separator.
fn top_colon(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (idx, ch) in s.char_indices() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                }
            }
            None => match ch {
                '"' | '\'' => quote = Some(ch),
                ':' => return Some(idx),
                _ => {}
            },
        }
    }
    None
}

fn strip_important(value: &mut String) -> bool {
    let lowered = value.to_ascii_lowercase();
    let trimmed = lowered.trim_end();
    if let Some(head) = trimmed.strip_suffix("!important") {
        value.truncate(head.trim_end().len());
        return true;
    }
    false
}

fn strip_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i] == '/' && i + 1 < b.len() && b[i + 1] == '*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

fn skip_ws(b: &[char], mut i: usize) -> usize {
    while i < b.len() && b[i].is_whitespace() {
        i += 1;
    }
    i
}

/// Read an at-rule at `start` (`b[start] == '@'`): its lowercased name, its
/// prelude (from the name to the terminating `;` or the `{`), its balanced
/// `{…}` body if it has one, and the index just past the rule.
fn read_at_rule(b: &[char], start: usize) -> (String, String, Option<String>, usize) {
    let mut i = start + 1;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == '-') {
        i += 1;
    }
    let name: String = b[start + 1..i]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase();
    let pre_start = i;
    while i < b.len() && b[i] != ';' && b[i] != '{' {
        i += 1;
    }
    let prelude: String = b[pre_start..i].iter().collect();
    if i >= b.len() || b[i] == ';' {
        return (name, prelude, None, (i + 1).min(b.len()));
    }
    let (body, next) = read_block(b, i);
    (name, prelude, Some(body), next)
}

/// Read a balanced `{…}` body whose `{` is at `open`. Returns the body text
/// and the index just past the closing `}` (an unterminated body reads to
/// end of sheet).
fn read_block(b: &[char], open: usize) -> (String, usize) {
    let mut i = open + 1;
    let body_start = i;
    let mut depth = 1u32;
    while i < b.len() {
        match b[i] {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            break;
        }
        i += 1;
    }
    let body: String = b[body_start..i].iter().collect();
    (body, (i + 1).min(b.len()))
}

/// Read one `prelude { body }` rule starting at `start`. Returns the prelude,
/// the body, and the index just past the closing `}`. `None` if there is no
/// `{` (trailing garbage at end of sheet).
fn read_rule(b: &[char], start: usize) -> Option<(String, String, usize)> {
    let mut i = start;
    while i < b.len() && b[i] != '{' {
        i += 1;
    }
    if i >= b.len() {
        return None;
    }
    let prelude: String = b[start..i].iter().collect();
    let (body, next) = read_block(b, i);
    Some((prelude, body, next))
}

#[cfg(test)]
mod tests;
