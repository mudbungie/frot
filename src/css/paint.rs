//! The painted facts Chrome's data-table test reads off a table's cells and
//! rows (`bl-e143`): which border sides are drawn, the background colour, and
//! `empty-cells`. Nothing here is laid out or painted by frot; `ax` reads it
//! (`ax/tree/table.rs`).
//!
//! - **Borders.** A side is drawn when its width is not zero and its style is
//!   not `none` or `hidden`. Width and style each come from the cascade's
//!   winner among the declarations that set them — `border`, `border-<side>`,
//!   `border-width` / `border-style` (one to four values, the box order), and
//!   `border-<side>-width` / `-style`. A shorthand that leaves width out sets
//!   it to `medium`, one that leaves style out sets `none`. The UA rule a
//!   `border` attribute draws is the table's, read in `ax`.
//! - **Background.** The colour of `background-color`, or of the `background`
//!   shorthand (the word in it that is no image, position, size, repeat, box
//!   or attachment keyword), else the `bgcolor` attribute. Colours are
//!   compared as written, lower-cased: `red` and `#f00` differ here.
//!   `transparent` and an absent colour are `None`.
//! - **`empty-cells`** is inherited; only `hide` is told apart.

use super::computed::Applied;
use crate::dom::Element;

/// What [`super::ComputedStyle::paint`] holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Paint {
    /// Drawn border sides: top, right, bottom, left.
    pub borders: [bool; 4],
    /// The background colour, lower-cased; `None` is transparent.
    pub background: Option<String>,
    /// Computed (inherited) `empty-cells: hide`.
    pub empty_cells_hide: bool,
}

const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];

impl Paint {
    pub fn of(applied: &[Applied], el: &Element, parent: &Paint) -> Paint {
        Paint {
            borders: std::array::from_fn(|i| drawn(applied, i)),
            background: background(applied, el),
            empty_cells_hide: match last(applied, &["empty-cells"]) {
                Some((_, v)) if !matches!(v.as_str(), "inherit" | "unset") => v == "hide",
                _ => parent.empty_cells_hide,
            },
        }
    }
}

/// The winning declaration among `names` (not on a pseudo-element), its
/// name and its value lower-cased and trimmed.
fn last(applied: &[Applied], names: &[&str]) -> Option<(String, String)> {
    applied
        .iter()
        .filter(|a| a.pseudo.is_none() && names.contains(&a.name.as_str()))
        .max_by_key(|a| a.key())
        .map(|a| (a.name.clone(), a.value.trim().to_ascii_lowercase()))
}

/// Whether border side `i` is drawn.
fn drawn(applied: &[Applied], i: usize) -> bool {
    let side = format!("border-{}", SIDES[i]);
    let part = |what: &str| {
        let (long, all) = (format!("{side}-{what}"), format!("border-{what}"));
        let (name, value) = last(applied, &["border", &side, &all, &long])?;
        let words = words(&value);
        Some(if name == all {
            // One to four values: top, right, bottom, left, the box order.
            let at = match (words.len(), i) {
                (1, _) => 0,
                (2, _) | (3, 3) => i % 2,
                (3, _) => i,
                _ => i,
            };
            words.get(at).map(|w| w.to_string())
        } else if name == long {
            Some(value)
        } else {
            words.into_iter().find(|w| is(what, w)).map(str::to_string)
        })
    };
    // No width written is `medium`; no style written is `none`.
    let width = part("width").flatten().is_none_or(|w| !zero(&w));
    let style = part("style").flatten();
    width && style.is_some_and(|s| !matches!(s.as_str(), "none" | "hidden"))
}

/// The words of a value, a function call (`rgb(0, 0, 0)`) being one word.
fn words(value: &str) -> Vec<&str> {
    let (mut out, mut depth, mut start) = (Vec::new(), 0usize, None);
    for (i, c) in value.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if c.is_ascii_whitespace() && depth == 0 => {
                out.extend(start.take().map(|s| &value[s..i]));
                continue;
            }
            _ => {}
        }
        start.get_or_insert(i);
    }
    out.extend(start.map(|s| &value[s..]));
    out
}

/// Whether `word` is a value of the border `what` (`width` or `style`).
fn is(what: &str, word: &str) -> bool {
    const STYLES: [&str; 10] = [
        "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset",
        "outset",
    ];
    match what {
        "style" => STYLES.contains(&word),
        _ => {
            matches!(word, "thin" | "medium" | "thick")
                || word.starts_with(|c: char| c.is_ascii_digit() || c == '.')
        }
    }
}

/// Whether a border width is zero.
fn zero(width: &str) -> bool {
    let num: String = width
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num.parse::<f64>().is_ok_and(|n| n == 0.0)
}

/// The background colour ([module docs](self)).
fn background(applied: &[Applied], el: &Element) -> Option<String> {
    let color = match last(applied, &["background", "background-color"]) {
        Some((name, v)) if name == "background-color" => Some(v),
        Some((_, v)) => words(&v)
            .into_iter()
            .find(|w| !painting_word(w))
            .map(str::to_string),
        None => el.attr("bgcolor").map(|v| v.trim().to_ascii_lowercase()),
    };
    color.filter(|c| !c.is_empty() && c != "transparent")
}

/// Whether a word of the `background` shorthand is something other than its
/// colour.
fn painting_word(w: &str) -> bool {
    const KEYWORDS: [&str; 22] = [
        "none",
        "repeat",
        "repeat-x",
        "repeat-y",
        "no-repeat",
        "space",
        "round",
        "scroll",
        "fixed",
        "local",
        "left",
        "right",
        "top",
        "bottom",
        "center",
        "auto",
        "cover",
        "contain",
        "border-box",
        "padding-box",
        "content-box",
        "/",
    ];
    KEYWORDS.contains(&w)
        || w.starts_with("url(")
        || w.contains("gradient(")
        || w.starts_with(|c: char| c.is_ascii_digit() || c == '.' || c == '-')
}

#[cfg(test)]
mod tests {
    use crate::css::compute;
    use crate::dom::Document;

    fn paint(html: &str) -> super::Paint {
        let doc = Document::parse(html);
        let id = *doc.find_by_tag("td").last().unwrap();
        compute(&doc).paint(id).clone()
    }

    fn cell(style: &str) -> String {
        format!("<style>td{{{style}}}</style><table><tr><td>x</td></tr></table>")
    }

    #[test]
    fn border_sides_follow_width_and_style() {
        for (style, want) in [
            ("", [false; 4]),
            ("border:1px solid", [true; 4]),
            ("border:solid", [true; 4]),
            ("border:1px", [false; 4]),
            ("border:0 solid", [false; 4]),
            ("border:.5px hidden", [false; 4]),
            ("border-style:solid", [true; 4]),
            (
                "border-style:solid;border-width:0 1px",
                [false, true, false, true],
            ),
            (
                "border-style:solid;border-width:0 1px 2px",
                [false, true, true, true],
            ),
            (
                "border-style:solid;border-width:0 0 1px 0",
                [false, false, true, false],
            ),
            (
                "border-top:thin dotted;border-left:2px",
                [true, false, false, false],
            ),
            (
                "border:1px solid;border-right-style:none;border-left-width:0px",
                [true, false, true, false],
            ),
            (
                "border-bottom:1px solid rgb(0, 0, 0)",
                [false, false, true, false],
            ),
        ] {
            assert_eq!(paint(&cell(style)).borders, want, "{style}");
        }
    }

    #[test]
    fn background_is_the_colour_word() {
        for (html, want) in [
            (cell(""), None),
            (cell("background:RED"), Some("red")),
            (
                cell("background:url(x.png) no-repeat 0 0 / cover #ABC"),
                Some("#abc"),
            ),
            (cell("background:url(x.png)"), None),
            (cell("background-color:transparent"), None),
            (cell("background:linear-gradient(red, blue)"), None),
            (
                "<table><tr><td bgcolor=' #F60 '>x</td></tr></table>".into(),
                Some("#f60"),
            ),
            (
                "<style>td{background:blue}</style><table><tr><td bgcolor=red>x</td></tr></table>"
                    .into(),
                Some("blue"),
            ),
        ] {
            assert_eq!(paint(&html).background.as_deref(), want, "{html}");
        }
    }

    #[test]
    fn empty_cells_is_inherited() {
        for (html, want) in [
            ("<table style='empty-cells:hide'><tr><td>x</td></tr></table>", true),
            ("<table style='empty-cells:HIDE'><tr><td style='empty-cells:show'>x</td></tr></table>", false),
            ("<table style='empty-cells:hide'><tr><td style='empty-cells:inherit'>x</td></tr></table>", true),
            ("<table style='empty-cells:hide'><tr><td style='empty-cells:unset'>x</td></tr></table>", true),
            ("<table><tr><td>x</td></tr></table>", false),
        ] {
            assert_eq!(paint(html).empty_cells_hide, want, "{html}");
        }
    }
}
