//! `sizes` and its lengths against Firefox 153.0esr (`bl-0e4a`): each row's
//! size is the one the rig's pick implies (the smallest `w` candidate not
//! below it, from 100w to 3200w), so the value is exact where the row says.

use super::{parse, size, Cand, Desc};

/// The size `sizes` resolves to, `auto` off.
fn px(sizes: &str) -> f64 {
    size(Some(sizes), None)
}

#[test]
fn sizes_resolve_as_firefox_resolved_them() {
    let rows: &[(&str, f64)] = &[
        ("calc(100vw - 2em)", 1248.0),
        ("min(50vw, 300px)", 300.0),
        ("max(10px, 30vw)", 384.0),
        ("clamp(100px, 20vw, 150px)", 150.0),
        (
            "(min-width: 1000px) and (max-width: 1300px) 150px, 10px",
            150.0,
        ),
        ("not (min-width: 2000px) 150px", 150.0),
        ("screen 150px", 1280.0),
        ("only (min-width: 1px) 150px", 1280.0),
        ("(min-width: 1000px) 150px 10px, 700px", 700.0),
        ("1in", 96.0),
        ("calc(2 * 100px)", 200.0),
        ("calc(100px / 2)", 50.0),
        ("0", 0.0),
        ("  150px  ", 150.0),
        ("CALC(150PX)", 150.0),
        ("1.5e2px", 150.0),
        ("(min-width:1000px)150px, 700px", 150.0),
        ("calc(50% )", 1280.0),
        ("150px,", 150.0),
        ("50vh", 360.0),
        ("vmin", 1280.0),
        ("300", 1280.0),
        ("-5px, 300px", 300.0),
        ("10ex", 1280.0),
        ("(min-width: 1000px)", 1280.0),
        ("(min-width: 1000px) (150px)", 1280.0),
    ];
    for (sizes, want) in rows {
        assert_eq!(px(sizes), *want, "{sizes}");
    }
    assert_eq!(size(None, None), 1280.0);
}

/// The rest of the length grammar, case by case.
#[test]
fn lengths_and_math_functions() {
    let rows: &[(&str, f64)] = &[
        ("2rem", 32.0),
        ("10vmin", 72.0),
        ("10vmax", 128.0),
        ("2.54cm", 96.0),
        ("25.4mm", 96.0),
        ("101.6q", 96.0),
        ("72pt", 96.0),
        ("6pc", 96.0),
        ("+5px", 5.0),
        (".5px", 0.5),
        ("1e1px", 10.0),
        ("1E+1px", 10.0),
        ("1e-1px", 0.1),
        ("calc(10px + 5px - 3px)", 12.0),
        ("calc((10px + 5px) * 2)", 30.0),
        ("calc(2 * (3 * 10px))", 60.0),
        ("max(1px, 2px, 3px)", 3.0),
        ("min(calc(4px * 2), 9px)", 8.0),
        ("-0px", 0.0),
    ];
    for (v, want) in rows {
        assert!((px(v) - want).abs() < 1e-9, "{v}: {}", px(v));
    }
    // Each malformed one falls back to the 100vw default.
    for v in [
        "5foo",
        "5.px",
        "1e",
        "calc(10px + 5)",
        "calc(10px * 5px)",
        "calc(10px / 5px)",
        "calc(10px / 0)",
        "calc(10px",
        "calc 10px",
        "min(5, 6)",
        "clamp(1px, 2px)",
        "foo(1px)",
        "calc(1px 2px)",
        "(10px)",
        "calc((1px)",
        "-",
        "calc(1px +)",
        "calc()",
        "max(1px,)",
        "calc(-5px)",
        "calc(5)",
        "calc((1px 2px))",
        "1px)",
        "calc(2 * )",
        "calc(2px / )",
        "calc(())",
    ] {
        assert_eq!(px(v), 1280.0, "{v}");
    }
}

#[test]
fn auto_is_a_size_only_when_given_one() {
    assert_eq!(size(Some("auto"), Some(200.0)), 200.0);
    assert_eq!(size(Some("AUTO, 50px"), None), 50.0);
}

#[test]
fn parse_keeps_order_and_drops_repeats() {
    let c = |url: &str, desc| Cand {
        url: url.into(),
        desc,
    };
    assert_eq!(
        parse("a 400w, b 2x,c,, d 400w ,e 2x, f 1x"),
        vec![
            c("a", Desc::W(400)),
            c("b", Desc::X(2.0)),
            c("c", Desc::X(1.0))
        ]
    );
    assert_eq!(parse("a (x y) 2x"), vec![]);
    assert_eq!(parse("a 2x (b"), vec![]);
    assert_eq!(parse(""), vec![]);
}
