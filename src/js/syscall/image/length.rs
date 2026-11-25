//! A `sizes` length (`bl-0e4a`): the `<source-size-value>` grammar Firefox
//! 153.0esr accepts, resolved to CSS pixels at the fixed viewport.
//!
//! Measured on the rig (js.md §7 "Images"): absolute units, `em`/`rem` at the
//! 16 px root, the four viewport units, a unitless `0`, and `calc()`/`min()`/
//! `max()`/`clamp()` over them, all case-insensitive; a negative length, a
//! unitless non-zero number and a percentage are invalid. `ex`/`ch` depend on
//! font metrics frot does not have, so they are invalid here (a residual).

use crate::layout::{VIEWPORT_HEIGHT, VIEWPORT_WIDTH};

/// `v` as CSS pixels, or `None` when it is not a valid non-negative length.
pub fn px(v: &str) -> Option<f64> {
    let mut p = Parser {
        s: v.trim().as_bytes(),
        i: 0,
    };
    let out = p.term()?;
    (p.i == p.s.len() && (out.len || out.v == 0.0) && out.v >= 0.0).then_some(out.v)
}

/// A value inside a math function: a length, or a bare number (a factor).
#[derive(Clone, Copy)]
struct Val {
    v: f64,
    len: bool,
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn at(&self, k: usize) -> Option<u8> {
        self.s.get(self.i + k).copied()
    }

    /// Consume a run of ASCII digits; whether there was one.
    fn digits(&mut self) -> bool {
        let at = self.i;
        while self.at(0).is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        self.i > at
    }

    /// The (ASCII) text from `start` to the cursor.
    fn text(&self, start: usize) -> String {
        self.s[start..self.i]
            .iter()
            .map(|&c| char::from(c))
            .collect()
    }

    fn ws(&mut self) {
        while self.at(0).is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        let hit = self.at(0) == Some(c);
        self.i += usize::from(hit);
        hit
    }

    /// One term: a math function, or a dimension / number.
    fn term(&mut self) -> Option<Val> {
        self.ws();
        let start = self.i;
        while self.at(0).is_some_and(|c| c.is_ascii_alphabetic()) {
            self.i += 1;
        }
        if self.i > start {
            return self.function(&self.text(start).to_ascii_lowercase());
        }
        self.dimension()
    }

    /// `name(` … `)`: `calc` takes a sum, the others a comma list of sums.
    fn function(&mut self, name: &str) -> Option<Val> {
        if !self.eat(b'(') {
            return None;
        }
        let mut args = vec![self.sum()?];
        while self.eat(b',') {
            args.push(self.sum()?);
        }
        if !self.eat(b')') || args.iter().any(|a| !a.len) {
            return None;
        }
        let v: Vec<f64> = args.iter().map(|a| a.v).collect();
        let v = match (name, v.as_slice()) {
            ("calc", [x]) => *x,
            ("min", [_, ..]) => v.iter().copied().fold(f64::INFINITY, f64::min),
            ("max", [_, ..]) => v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ("clamp", [lo, x, hi]) => x.min(*hi).max(*lo),
            _ => return None,
        };
        Some(Val { v, len: true })
    }

    /// `product (('+' | '-') product)*`, the operands of one kind.
    fn sum(&mut self) -> Option<Val> {
        let mut acc = self.product()?;
        loop {
            let sign = if self.eat(b'+') {
                1.0
            } else if self.eat(b'-') {
                -1.0
            } else {
                return Some(acc);
            };
            let rhs = self.product()?;
            if rhs.len != acc.len {
                return None;
            }
            acc.v += sign * rhs.v;
        }
    }

    /// `factor (('*' | '/') factor)*`: at most one side a length, never a
    /// length divisor.
    fn product(&mut self) -> Option<Val> {
        let mut acc = self.factor()?;
        loop {
            if self.eat(b'*') {
                let rhs = self.factor()?;
                if acc.len && rhs.len {
                    return None;
                }
                acc = Val {
                    v: acc.v * rhs.v,
                    len: acc.len || rhs.len,
                };
            } else if self.eat(b'/') {
                let rhs = self.factor()?;
                if rhs.len || rhs.v == 0.0 {
                    return None;
                }
                acc.v /= rhs.v;
            } else {
                return Some(acc);
            }
        }
    }

    fn factor(&mut self) -> Option<Val> {
        if self.eat(b'(') {
            let v = self.sum()?;
            return self.eat(b')').then_some(v);
        }
        self.term()
    }

    /// A CSS number with an optional unit. A bare number is a factor; only
    /// [`px`] lets a bare `0` stand as a length.
    fn dimension(&mut self) -> Option<Val> {
        let start = self.i;
        if matches!(self.at(0), Some(b'-' | b'+')) {
            self.i += 1;
        }
        let int = self.digits();
        let frac = self.at(0) == Some(b'.') && self.at(1).is_some_and(|c| c.is_ascii_digit());
        if frac {
            self.i += 1;
            self.digits();
        }
        if !(int || frac) {
            return None;
        }
        let exp = matches!(self.at(0), Some(b'e' | b'E'));
        let signed = usize::from(matches!(self.at(1), Some(b'-' | b'+')));
        if exp && self.at(1 + signed).is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1 + signed;
            self.digits();
        }
        let n: f64 = self.text(start).parse().unwrap_or(f64::NAN);
        let unit_at = self.i;
        while self.at(0).is_some_and(|c| c.is_ascii_alphabetic()) {
            self.i += 1;
        }
        let unit = self.text(unit_at).to_ascii_lowercase();
        if unit.is_empty() {
            return Some(Val { v: n, len: false });
        }
        let (w, h) = (f64::from(VIEWPORT_WIDTH), f64::from(VIEWPORT_HEIGHT));
        let scale = match unit.as_str() {
            "px" => 1.0,
            "em" | "rem" => 16.0,
            "vw" => w / 100.0,
            "vh" => h / 100.0,
            "vmin" => w.min(h) / 100.0,
            "vmax" => w.max(h) / 100.0,
            "in" => 96.0,
            "cm" => 96.0 / 2.54,
            "mm" => 96.0 / 25.4,
            "q" => 96.0 / 101.6,
            "pt" => 96.0 / 72.0,
            "pc" => 16.0,
            _ => return None,
        };
        Some(Val {
            v: n * scale,
            len: true,
        })
    }
}
