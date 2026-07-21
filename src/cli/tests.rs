use super::*;

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn no_args_is_no_args() {
    assert_eq!(parse(&[]), Err(CliError::NoArgs));
}

#[test]
fn url_and_out_only() {
    let a = parse(&argv(&["https://x/", "--out", "text"])).unwrap();
    assert_eq!(a.url, "https://x/");
    assert_eq!(a.out, View::Text);
    assert!(!a.css);
    assert!(a.headers.is_empty());
}

#[test]
fn out_equals_form() {
    let a = parse(&argv(&["https://x/", "--out=dom"])).unwrap();
    assert_eq!(a.out, View::Dom);
}

#[test]
fn css_flag() {
    let a = parse(&argv(&["--css", "https://x/", "--out", "ax"])).unwrap();
    assert!(a.css);
    assert_eq!(a.out, View::Ax);
}

#[test]
fn flags_after_url_order_independent() {
    let a = parse(&argv(&["https://x/", "--css", "--out", "links"])).unwrap();
    assert!(a.css);
    assert_eq!(a.out, View::Links);
}

#[test]
fn js_flag_accepted() {
    let a = parse(&argv(&["https://x/", "--js", "--out", "text"])).unwrap();
    assert!(a.js);
    assert!(!a.css);
}

#[test]
fn js_flag_defaults_off_and_rejects_duplicates() {
    let a = parse(&argv(&["https://x/", "--out", "text"])).unwrap();
    assert!(!a.js);
    assert_eq!(
        parse(&argv(&["https://x/", "--js", "--js", "--out", "text"])),
        Err(CliError::DuplicateFlag("--js".into()))
    );
}

#[test]
fn js_errors_requires_js() {
    // --js-errors only shapes the `js` block, which exists only under --js; bare
    // it is a usage error (exit 2 at the run layer), like -H on a file:// URL.
    assert_eq!(
        parse(&argv(&["https://x/", "--js-errors", "--out", "text"])),
        Err(CliError::JsErrorsWithoutJs)
    );
    // With --js it is accepted and defaults off without it.
    let a = parse(&argv(&[
        "https://x/",
        "--js",
        "--js-errors",
        "--out",
        "text",
    ]))
    .unwrap();
    assert!(a.js && a.js_errors);
    let b = parse(&argv(&["https://x/", "--js", "--out", "text"])).unwrap();
    assert!(b.js && !b.js_errors);
}

#[test]
fn js_errors_rejects_duplicates() {
    assert_eq!(
        parse(&argv(&[
            "https://x/",
            "--js",
            "--js-errors",
            "--js-errors",
            "--out",
            "text"
        ])),
        Err(CliError::DuplicateFlag("--js-errors".into()))
    );
}

#[test]
fn bboxes_view_now_parses() {
    for form in [
        argv(&["https://x/", "--out", "bboxes"]),
        argv(&["https://x/", "--out=bboxes"]),
    ] {
        assert_eq!(parse(&form).unwrap().out, View::Bboxes);
    }
}

#[test]
fn missing_url() {
    assert_eq!(parse(&argv(&["--out", "text"])), Err(CliError::NoUrl));
}

#[test]
fn missing_out() {
    assert_eq!(parse(&argv(&["https://x/"])), Err(CliError::NoOut));
}

#[test]
fn unknown_view() {
    assert_eq!(
        parse(&argv(&["https://x/", "--out", "wat"])),
        Err(CliError::UnknownView("wat".into()))
    );
    assert_eq!(
        parse(&argv(&["https://x/", "--out=wat"])),
        Err(CliError::UnknownView("wat".into()))
    );
}

#[test]
fn unknown_flag_long_and_short() {
    assert_eq!(
        parse(&argv(&["https://x/", "--out", "text", "--zzz"])),
        Err(CliError::UnknownFlag("--zzz".into()))
    );
    assert_eq!(
        parse(&argv(&["-x", "https://x/", "--out", "text"])),
        Err(CliError::UnknownFlag("-x".into()))
    );
}

#[test]
fn missing_value_for_out() {
    assert_eq!(
        parse(&argv(&["https://x/", "--out"])),
        Err(CliError::MissingValue("--out".into()))
    );
}

#[test]
fn duplicate_out() {
    assert_eq!(
        parse(&argv(&["https://x/", "--out", "text", "--out", "dom"])),
        Err(CliError::DuplicateFlag("--out".into()))
    );
    assert_eq!(
        parse(&argv(&["https://x/", "--out=text", "--out=dom"])),
        Err(CliError::DuplicateFlag("--out".into()))
    );
}

#[test]
fn duplicate_css() {
    assert_eq!(
        parse(&argv(&["--css", "--css", "https://x/", "--out", "text"])),
        Err(CliError::DuplicateFlag("--css".into()))
    );
}

#[test]
fn extra_positional_rejected() {
    assert_eq!(
        parse(&argv(&["https://x/", "extra", "--out", "text"])),
        Err(CliError::ExtraPositional("extra".into()))
    );
}

#[test]
fn help_and_version_short_and_long() {
    for flag in ["-h", "--help"] {
        let e = parse(&argv(&[flag])).unwrap_err();
        assert_eq!(e, CliError::Help);
        assert!(e.is_help_or_version());
    }
    for flag in ["-V", "--version"] {
        let e = parse(&argv(&[flag])).unwrap_err();
        assert_eq!(e, CliError::Version);
        assert!(e.is_help_or_version());
    }
}

mod more;
