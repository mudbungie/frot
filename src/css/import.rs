//! `@import` discovery: which other sheets a stylesheet text pulls into the
//! screen cascade.
//!
//! The engine never fetches; the gather phase (`run::gather`) asks here which
//! sheets a text imports, fetches them, and hands the cascade the imported
//! text ahead of the importer — an `@import` stands for the imported rules at
//! the top of the sheet, so they lose source-order ties to the sheet's own.
//!
//! Supported: `@import "x";` and `@import url(x);` (the `url()` argument
//! quoted or bare), optionally followed by a media-query list, which is
//! evaluated by the one media authority ([`super::media`]) — a non-matching
//! import is not reported, exactly as a non-matching `<link media>` is not
//! gathered. `layer`, `layer(…)` and `supports(…)` are not media queries, so
//! an import carrying one is `not all` and is not reported. An `@import` is
//! honoured only where the grammar allows it: before every rule other than
//! `@charset` and a body-less `@layer`.

use super::escape::{unquote, Quoting};
use super::media;
use super::parse::{read_at_rule, skip_ws, strip_comments};

/// The targets of the `@import` rules of `src` that apply at the fixed
/// viewport, in source order and as written (unresolved).
pub fn imports(src: &str) -> Vec<String> {
    let chars: Vec<char> = strip_comments(src).chars().collect();
    let mut out = Vec::new();
    let mut i = skip_ws(&chars, 0);
    while i < chars.len() && chars[i] == '@' {
        let (name, prelude, body, next) = read_at_rule(&chars, i);
        match (name.as_str(), body) {
            ("import", None) => out.extend(target(prelude.trim())),
            ("charset" | "layer", None) => {}
            _ => break,
        }
        i = skip_ws(&chars, next);
    }
    out
}

/// The characters of `s` outside string literals, with their byte offsets.
fn structural(s: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut quoting = Quoting::default();
    s.char_indices().filter(move |&(_, c)| !quoting.feed(c))
}

/// The target of one `@import` prelude, if its form is supported and its
/// media-query list (everything after the target) matches.
fn target(prelude: &str) -> Option<String> {
    let (url, rest) = if prelude
        .get(..4)
        .is_some_and(|p| p.eq_ignore_ascii_case("url("))
    {
        let arg = &prelude[4..];
        let close = structural(arg).find(|&(_, c)| c == ')').map(|(i, _)| i)?;
        let inner = arg[..close].trim();
        let url = if inner.starts_with(['"', '\'']) {
            unquote(inner)
        } else {
            inner.to_string()
        };
        (url, &arg[close + 1..])
    } else if prelude.starts_with(['"', '\'']) {
        let end = structural(prelude).next().map_or(prelude.len(), |(i, _)| i);
        (unquote(prelude), &prelude[end..])
    } else {
        return None;
    };
    media::matches(rest).then_some(url)
}

#[cfg(test)]
mod tests;
