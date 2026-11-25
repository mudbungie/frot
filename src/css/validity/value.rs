//! An `<input>`'s value against its own constraints (`bl-51e9`), all skipped
//! for an empty value: the type (`email`, `url`), `pattern` (the engine's own
//! `RegExp`, [`crate::js::engine::pattern_matches`]; one that does not
//! compile is ignored), the range (`min`/`max` on `number` and the date and
//! time types — a `range` input is sanitized into its range, so it is always
//! in it, and an empty value of any of them is in range even with no `min` or
//! `max`) and the step (`number`, default 1; `time` and `datetime-local`,
//! default 60 seconds — `12:30:15.5` mismatches; the base is `min`, else the
//! `value` attribute, else 0; `step=any` has none). An explicit `step` on the
//! other date types is not checked: a residual.

use super::super::control::{number, sanitized as text};
use crate::dom::Element;

/// The types an `<input>` may be; any other spelling is `text`.
const KINDS: [&str; 22] = [
    "text",
    "search",
    "tel",
    "url",
    "email",
    "password",
    "date",
    "month",
    "week",
    "time",
    "datetime-local",
    "number",
    "range",
    "color",
    "checkbox",
    "radio",
    "file",
    "submit",
    "image",
    "reset",
    "button",
    "hidden",
];
/// The types `pattern` applies to.
const PATTERNED: [&str; 6] = ["text", "search", "tel", "url", "email", "password"];

/// The input's type, `text` when it names none.
pub(super) fn kind(el: &Element) -> String {
    let raw = super::input_type(el);
    let known = KINDS.contains(&raw.as_str());
    if known {
        raw
    } else {
        "text".to_owned()
    }
}

/// The value as the type sanitizes it: a date or time that does not parse is
/// empty, like a number that does not.
pub(super) fn sanitized(el: &Element, kind: &str) -> String {
    let value = text(kind, el.attr("value").unwrap_or_default());
    let dated = !matches!(kind, "number" | "range") && key(kind, &value).is_none();
    if dated && RANGED.contains(&kind) {
        String::new()
    } else {
        value
    }
}

/// Whether the value breaks its type, `pattern`, range or step.
pub(super) fn bad(el: &Element, kind: &str) -> bool {
    let value = sanitized(el, kind);
    let parts: Vec<&str> = match kind == "email" && el.attr("multiple").is_some() {
        true => value
            .split(',')
            .map(|p| p.trim_matches(|c: char| c.is_ascii_whitespace()))
            .collect(),
        false => vec![&value],
    };
    let mismatch = |p: &str| match kind {
        "email" => !email(p),
        "url" => url::Url::parse(p).is_err(),
        _ => false,
    };
    let pattern = el.attr("pattern").filter(|_| PATTERNED.contains(&kind));
    let unmatched = |p: &str| {
        pattern.is_some_and(|re| crate::js::engine::pattern_matches(re, p) == Some(false))
    };
    let broken = parts.iter().any(|p| mismatch(p) || unmatched(p));
    !value.is_empty() && (broken || range(el, kind) == Some(false) || step(el, kind, &value))
}

/// The types with range limitations, and how each orders its values.
const RANGED: [&str; 7] = [
    "number",
    "range",
    "date",
    "month",
    "week",
    "time",
    "datetime-local",
];

/// `Some(in range)` when the type has range limitations and `min` or `max`
/// sets one (a `range` input always has them); an empty value is in range.
pub(super) fn range(el: &Element, kind: &str) -> Option<bool> {
    if kind == "range" {
        return Some(true);
    }
    let bound = |name: &str| el.attr(name).and_then(|s| key(kind, s));
    let (lo, hi) = (bound("min"), bound("max"));
    let value = key(kind, &sanitized(el, kind));
    let limited = RANGED.contains(&kind) && (lo.is_some() || hi.is_some() || value.is_none());
    let above = |v: f64| lo.is_none_or(|l| v >= l);
    let below = |v: f64| hi.is_none_or(|h| v <= h);
    // Only `time` may wrap round midnight: `max` before `min` is a reversed range.
    let reversed = kind == "time" && lo > hi && hi.is_some();
    let within = |v| {
        if reversed {
            above(v) || below(v)
        } else {
            above(v) && below(v)
        }
    };
    limited.then(|| value.is_none_or(within))
}

fn step(el: &Element, kind: &str, value: &str) -> bool {
    let (default, stepped) = match kind {
        "number" => (1.0, true),
        k => (60.0, matches!(k, "time" | "datetime-local")),
    };
    let num = |s: &str| key(kind, s);
    let any = el
        .attr("step")
        .is_some_and(|s| s.trim().eq_ignore_ascii_case("any"));
    let step = el
        .attr("step")
        .and_then(|s| key("number", s))
        .filter(|&s| s > 0.0);
    let base = el
        .attr("min")
        .and_then(num)
        .or(el.attr("value").and_then(num));
    let q = (num(value).unwrap_or_default() - base.unwrap_or_default()) / step.unwrap_or(default);
    stepped && !any && (q - q.round()).abs() > 1e-9
}

/// A value of `kind` as a number that orders like it, or `None` when it is
/// not one.
fn key(kind: &str, s: &str) -> Option<f64> {
    let (date, rest) = s.split_at(s.find(['T', ' ']).unwrap_or(s.len()));
    match kind {
        "date" => day(s),
        "month" => {
            let (y, m) = s.split_once('-')?;
            Some(year(y)? * 12.0 + within(m, 1, 12)?)
        }
        "week" => {
            let (y, w) = s.split_once("-W")?;
            Some(year(y)? * 54.0 + within(w, 1, 53)?)
        }
        "time" => clock(s),
        "datetime-local" => Some(day(date)? * 86_400.0 + clock(rest.get(1..)?)?),
        _ => number(s).then(|| s.parse().ok()).flatten(),
    }
}

fn day(s: &str) -> Option<f64> {
    let (y, rest) = s.split_once('-')?;
    let (m, d) = rest.split_once('-')?;
    let (y, m) = (year(y)?, within(m, 1, 12)?);
    let leap = y % 4.0 == 0.0 && (y % 100.0 != 0.0 || y % 400.0 == 0.0);
    let days = [
        31,
        28 + u32::from(leap),
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    Some(y * 372.0 + m * 31.0 + within(d, 1, days[m as usize - 1])?)
}

/// `HH:MM`, `HH:MM:SS` or `HH:MM:SS.fff` as seconds.
fn clock(s: &str) -> Option<f64> {
    let (h, rest) = s.split_once(':')?;
    let (m, sec) = rest
        .split_once(':')
        .map_or((rest, None), |(m, s)| (m, Some(s)));
    let (h, m) = (within(h, 0, 23)?, within(m, 0, 59)?);
    let sec = match sec {
        Some(rest) => {
            let (whole, frac) = rest.split_once('.').unwrap_or((rest, ""));
            let fine = frac.len() <= 3 && frac.bytes().all(|b| b.is_ascii_digit());
            let frac: f64 = format!("0.{frac}0").parse().ok().filter(|_| fine)?;
            within(whole, 0, 59)? + frac
        }
        None => 0.0,
    };
    Some(h * 3600.0 + m * 60.0 + sec)
}

fn year(s: &str) -> Option<f64> {
    (s.len() >= 4).then_some(())?;
    within(s, 1, u32::MAX)
}

/// Two or more ASCII digits (`year` checks its own width) naming `lo..=hi`.
fn within(s: &str, lo: u32, hi: u32) -> Option<f64> {
    let digits = s.len() >= 2 && s.bytes().all(|b| b.is_ascii_digit());
    let n: u32 = s.parse().ok().filter(|_| digits)?;
    (lo..=hi).contains(&n).then_some(f64::from(n))
}

/// HTML's valid email address: a local part of the allowed characters, `@`,
/// then dot-separated labels of letters, digits and inner hyphens.
fn email(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    let allowed = |c: char| c.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(c);
    let label = |l: &str| {
        let inner = l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        (1..=63).contains(&l.len()) && inner && !l.starts_with('-') && !l.ends_with('-')
    };
    !local.is_empty() && local.chars().all(allowed) && domain.split('.').all(label)
}
