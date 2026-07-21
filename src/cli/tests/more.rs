//! Overflow tests split from the parent module to hold the 300-line cap.
use super::*;

#[test]
fn non_help_is_not_help_or_version() {
    assert!(!CliError::NoArgs.is_help_or_version());
    assert!(!CliError::NoUrl.is_help_or_version());
    assert!(!CliError::DuplicateFlag("--js".into()).is_help_or_version());
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
        CliError::JsErrorsWithoutJs,
        CliError::NoUrl,
        CliError::NoOut,
        CliError::ExtraPositional("oops".into()),
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
        "https://x/",
        "-H",
        "B: 2",
        "-H",
        "A: 1",
        "-H",
        "A: 3",
        "--out",
        "text",
    ]))
    .unwrap();
    let got: Vec<String> = a
        .headers
        .iter()
        .map(|(n, v)| format!("{}={}", n, v))
        .collect();
    assert_eq!(got, ["B=2", "A=1", "A=3"]);
}

#[test]
fn header_value_may_contain_colons() {
    let a = parse(&argv(&[
        "https://x/",
        "-H",
        "Authorization: Bearer a:b:c",
        "--out",
        "text",
    ]))
    .unwrap();
    assert_eq!(
        a.headers[0],
        ("Authorization".into(), "Bearer a:b:c".into())
    );
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
        // The `--header=` spelling rejects identically: one validator, both forms.
        assert_eq!(
            parse(&argv(&[
                "https://x/",
                &format!("--header={raw}"),
                "--out",
                "text"
            ])),
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
