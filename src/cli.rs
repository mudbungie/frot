//! Hand-rolled argv parser. clap is avoided for binary-size reasons.
//!
//! Surface: `frot <url> [-H "Name: value"] [--css] [--js] --out <view>`.

use crate::envelope::View;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub url: String,
    pub css: bool,
    /// Run page scripts in the embedded engine before the rest of the pipeline
    /// consumes the (now post-JS) document (`docs/design/js.md` §9).
    pub js: bool,
    /// Surface the bounded `js.messages` error detail (`docs/design/js.md` §10).
    /// Requires `--js`; the count `js.errors` is emitted with or without it.
    pub js_errors: bool,
    pub out: View,
    /// Request headers, in argv order. Sent with the page request and with
    /// same-origin `--css` subfetches; see `fetch`.
    pub headers: Vec<(String, String)>,
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
    BadHeader(String),
    HeadersWithFile,
    JsErrorsWithoutJs,
    NoUrl,
    NoOut,
    ExtraPositional(String),
}

impl CliError {
    pub fn is_help_or_version(&self) -> bool {
        matches!(self, CliError::Help | CliError::Version)
    }
}

pub const USAGE: &str = "usage: frot <url> [-H \"Name: value\"] [--css] [--js] [--js-errors] --out <dom|text|ax|links|forms|bboxes|meta>";

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
                write!(
                    f,
                    "unknown view: {} (choose from {})\n{}",
                    s,
                    view_list(),
                    USAGE
                )
            }
            CliError::DuplicateFlag(s) => write!(f, "duplicate flag: {}\n{}", s, USAGE),
            CliError::BadHeader(s) => {
                write!(f, "bad header (want \"Name: value\"): {}\n{}", s, USAGE)
            }
            CliError::HeadersWithFile => {
                write!(f, "-H cannot apply to a file:// URL\n{}", USAGE)
            }
            CliError::JsErrorsWithoutJs => {
                write!(f, "--js-errors requires --js\n{}", USAGE)
            }
            CliError::NoUrl => write!(f, "missing <url>\n{}", USAGE),
            CliError::NoOut => write!(f, "missing --out <view>\n{}", USAGE),
            CliError::ExtraPositional(s) => write!(f, "unexpected argument: {}\n{}", s, USAGE),
        }
    }
}

/// `"Name: value"` → `(Name, value)`. The name must be non-empty and
/// whitespace-free; the value is trimmed.
fn parse_header(raw: &str) -> Result<(String, String), CliError> {
    let (name, value) = raw
        .split_once(':')
        .ok_or_else(|| CliError::BadHeader(raw.into()))?;
    let name = name.trim();
    if name.is_empty() || name.contains(char::is_whitespace) {
        return Err(CliError::BadHeader(raw.into()));
    }
    Ok((name.to_string(), value.trim().to_string()))
}

pub fn parse(argv: &[String]) -> Result<Args, CliError> {
    if argv.is_empty() {
        return Err(CliError::NoArgs);
    }

    let mut url: Option<String> = None;
    let mut css = false;
    let mut js = false;
    let mut js_errors = false;
    let mut out: Option<View> = None;
    let mut headers: Vec<(String, String)> = Vec::new();

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
            "--js-errors" => {
                if js_errors {
                    return Err(CliError::DuplicateFlag("--js-errors".into()));
                }
                js_errors = true;
            }
            "-H" | "--header" => {
                i += 1;
                let v = argv
                    .get(i)
                    .ok_or_else(|| CliError::MissingValue("-H".into()))?;
                headers.push(parse_header(v)?);
            }
            s if s.starts_with("--header=") => {
                headers.push(parse_header(&s[9..])?);
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
    // `--js-errors` only shapes the `js` block, which exists only under `--js`;
    // an accepted flag must do something — reject the bare combination (mirrors
    // the -H/file:// gating below).
    if js_errors && !js {
        return Err(CliError::JsErrorsWithoutJs);
    }
    // Under the same-origin rule headers could never be sent from a file://
    // page, and an accepted flag must do something — reject the combination.
    let is_file = url
        .get(..5)
        .is_some_and(|p| p.eq_ignore_ascii_case("file:"));
    if is_file && !headers.is_empty() {
        return Err(CliError::HeadersWithFile);
    }
    Ok(Args {
        url,
        css,
        js,
        js_errors,
        out,
        headers,
    })
}

#[cfg(test)]
mod tests;
