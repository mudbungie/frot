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
fn js_flag_rejected_as_unimplemented() {
    assert_eq!(
        parse(&argv(&["https://x/", "--js", "--out", "text"])),
        Err(CliError::Unimplemented(JS_UNIMPLEMENTED.into()))
    );
    assert!(JS_UNIMPLEMENTED.contains("Phase 4"));
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

#[test]
fn non_help_is_not_help_or_version() {
    assert!(!CliError::NoArgs.is_help_or_version());
    assert!(!CliError::NoUrl.is_help_or_version());
    assert!(!CliError::Unimplemented("x".into()).is_help_or_version());
}

#[test]
fn display_each_error_variant() {
    let cases = [
        CliError::NoArgs,
        CliError::Help,
        CliError::Version,
        CliError::UnknownFlag("--x".into()),
        CliError::MissingValue("--out".into()),
        CliError::UnknownView("zz".into()),
        CliError::DuplicateFlag("--css".into()),
        CliError::BadHeader("nope".into()),
        CliError::HeadersWithFile,
        CliError::NoUrl,
        CliError::NoOut,
        CliError::ExtraPositional("oops".into()),
        CliError::Unimplemented("--js: nope".into()),
    ];
    for c in cases {
        let s = format!("{}", c);
        assert!(!s.is_empty(), "{:?} rendered empty", c);
    }
}

#[test]
fn lone_hyphen_is_a_url() {
    let a = parse(&argv(&["-", "--out", "text"])).unwrap();
    assert_eq!(a.url, "-");
}

#[test]
fn header_short_long_and_equals_forms() {
    for form in [
        argv(&["https://x/", "-H", "X-A: 1", "--out", "text"]),
        argv(&["https://x/", "--header", "X-A: 1", "--out", "text"]),
        argv(&["https://x/", "--header=X-A: 1", "--out", "text"]),
    ] {
        let a = parse(&form).unwrap();
        assert_eq!(a.headers, vec![("X-A".to_string(), "1".to_string())]);
    }
}

#[test]
fn multiple_headers_preserve_argv_order() {
    let a = parse(&argv(&[
        "https://x/", "-H", "B: 2", "-H", "A: 1", "-H", "A: 3", "--out", "text",
    ]))
    .unwrap();
    let got: Vec<String> = a.headers.iter().map(|(n, v)| format!("{}={}", n, v)).collect();
    assert_eq!(got, ["B=2", "A=1", "A=3"]);
}

#[test]
fn header_value_may_contain_colons() {
    let a = parse(&argv(&[
        "https://x/", "-H", "Authorization: Bearer a:b:c", "--out", "text",
    ]))
    .unwrap();
    assert_eq!(a.headers[0], ("Authorization".into(), "Bearer a:b:c".into()));
}

#[test]
fn bad_headers_rejected() {
    for raw in ["no-colon-here", ": empty-name", "spaced name: v"] {
        assert_eq!(
            parse(&argv(&["https://x/", "-H", raw, "--out", "text"])),
            Err(CliError::BadHeader(raw.into())),
            "raw was {:?}",
            raw
        );
    }
}

#[test]
fn missing_value_for_header() {
    assert_eq!(
        parse(&argv(&["https://x/", "--out", "text", "-H"])),
        Err(CliError::MissingValue("-H".into()))
    );
}

#[test]
fn headers_with_file_url_rejected_case_insensitively() {
    for u in ["file:///tmp/x.html", "FILE:///tmp/x.html"] {
        assert_eq!(
            parse(&argv(&[u, "-H", "X-A: 1", "--out", "text"])),
            Err(CliError::HeadersWithFile)
        );
    }
}

#[test]
fn file_url_without_headers_is_fine() {
    let a = parse(&argv(&["file:///tmp/x.html", "--out", "text"])).unwrap();
    assert_eq!(a.url, "file:///tmp/x.html");
}
