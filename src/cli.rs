//! Hand-rolled argv parser. clap is avoided for binary-size reasons.
//!
//! Surface: `frot <url> [--css] [--js] --out <view>`.

use crate::envelope::View;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub url: String,
    pub css: bool,
    pub js: bool,
    pub out: View,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    NoArgs,
    Help,
    Version,
    UnknownFlag(String),
    MissingValue(String),
    UnknownView(String),
    DuplicateFlag(String),
    NoUrl,
    NoOut,
    ExtraPositional(String),
}

impl CliError {
    pub fn is_help_or_version(&self) -> bool {
        matches!(self, CliError::Help | CliError::Version)
    }
}

pub const USAGE: &str =
    "usage: frot <url> [--css] [--js] --out <dom|text|ax|links|forms|bboxes|meta>";

fn view_list() -> String {
    View::ALL
        .iter()
        .map(|(n, _)| *n)
        .collect::<Vec<_>>()
        .join("|")
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::NoArgs => write!(f, "no arguments\n{}", USAGE),
            CliError::Help => write!(f, "{}", USAGE),
            CliError::Version => write!(f, "frot {}", env!("CARGO_PKG_VERSION")),
            CliError::UnknownFlag(s) => write!(f, "unknown flag: {}\n{}", s, USAGE),
            CliError::MissingValue(s) => write!(f, "missing value for {}\n{}", s, USAGE),
            CliError::UnknownView(s) => {
                write!(f, "unknown view: {} (choose from {})\n{}", s, view_list(), USAGE)
            }
            CliError::DuplicateFlag(s) => write!(f, "duplicate flag: {}\n{}", s, USAGE),
            CliError::NoUrl => write!(f, "missing <url>\n{}", USAGE),
            CliError::NoOut => write!(f, "missing --out <view>\n{}", USAGE),
            CliError::ExtraPositional(s) => write!(f, "unexpected argument: {}\n{}", s, USAGE),
        }
    }
}

pub fn parse(argv: &[String]) -> Result<Args, CliError> {
    if argv.is_empty() {
        return Err(CliError::NoArgs);
    }

    let mut url: Option<String> = None;
    let mut css = false;
    let mut js = false;
    let mut out: Option<View> = None;

    let mut i = 0;
    while i < argv.len() {
        let a = &argv[i];
        match a.as_str() {
            "-h" | "--help" => return Err(CliError::Help),
            "-V" | "--version" => return Err(CliError::Version),
            "--css" => {
                if css {
                    return Err(CliError::DuplicateFlag("--css".into()));
                }
                css = true;
            }
            "--js" => {
                if js {
                    return Err(CliError::DuplicateFlag("--js".into()));
                }
                js = true;
            }
            "--out" => {
                if out.is_some() {
                    return Err(CliError::DuplicateFlag("--out".into()));
                }
                i += 1;
                let v = argv
                    .get(i)
                    .ok_or_else(|| CliError::MissingValue("--out".into()))?;
                out = Some(View::parse(v).ok_or_else(|| CliError::UnknownView(v.clone()))?);
            }
            s if s.starts_with("--out=") => {
                if out.is_some() {
                    return Err(CliError::DuplicateFlag("--out".into()));
                }
                let v = &s[6..];
                out = Some(View::parse(v).ok_or_else(|| CliError::UnknownView(v.into()))?);
            }
            s if s.starts_with("--") => return Err(CliError::UnknownFlag(s.into())),
            s if s.starts_with('-') && s.len() > 1 => return Err(CliError::UnknownFlag(s.into())),
            _ => {
                if url.is_some() {
                    return Err(CliError::ExtraPositional(a.clone()));
                }
                url = Some(a.clone());
            }
        }
        i += 1;
    }

    let url = url.ok_or(CliError::NoUrl)?;
    let out = out.ok_or(CliError::NoOut)?;
    Ok(Args { url, css, js, out })
}

#[cfg(test)]
mod tests {
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
        assert!(!a.js);
    }

    #[test]
    fn out_equals_form() {
        let a = parse(&argv(&["https://x/", "--out=dom"])).unwrap();
        assert_eq!(a.out, View::Dom);
    }

    #[test]
    fn css_and_js_flags() {
        let a = parse(&argv(&["--css", "--js", "https://x/", "--out", "ax"])).unwrap();
        assert!(a.css);
        assert!(a.js);
        assert_eq!(a.out, View::Ax);
    }

    #[test]
    fn flags_after_url_order_independent() {
        let a = parse(&argv(&["https://x/", "--css", "--out", "links", "--js"])).unwrap();
        assert!(a.css && a.js);
        assert_eq!(a.out, View::Links);
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
    fn duplicate_css_and_js() {
        assert_eq!(
            parse(&argv(&["--css", "--css", "https://x/", "--out", "text"])),
            Err(CliError::DuplicateFlag("--css".into()))
        );
        assert_eq!(
            parse(&argv(&["--js", "--js", "https://x/", "--out", "text"])),
            Err(CliError::DuplicateFlag("--js".into()))
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
}
