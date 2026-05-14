//! frot — take an impression of a web page.
//!
//! See `VISION.md`. The library exposes the envelope schema, the CLI parser,
//! and a top-level [`run`] entry point that the binary delegates to.

pub mod cli;
pub mod dom;
pub mod envelope;
pub mod fetch;
pub mod views;

/// Run the CLI against an argv slice. Returns the process exit code.
///
/// * `0` — help or version requested.
/// * `2` — usage error, or pre-Phase-0 stub that has not been wired yet.
///
/// Phase 0 will replace the `Ok(_)` arm with the real fetch/parse/view path.
pub fn run(argv: &[String]) -> u8 {
    match cli::parse(argv) {
        Ok(_args) => {
            eprintln!("frot: phase 0 in progress; fetch/parse path not yet wired");
            2
        }
        Err(e) if e.is_help_or_version() => {
            println!("{}", e);
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn run_help_is_exit_zero() {
        assert_eq!(run(&argv(&["--help"])), 0);
    }

    #[test]
    fn run_version_is_exit_zero() {
        assert_eq!(run(&argv(&["--version"])), 0);
    }

    #[test]
    fn run_no_args_is_exit_two() {
        assert_eq!(run(&argv(&[])), 2);
    }

    #[test]
    fn run_valid_args_stub_exits_two() {
        assert_eq!(run(&argv(&["https://x/", "--out", "text"])), 2);
    }

    #[test]
    fn run_usage_error_is_exit_two() {
        assert_eq!(run(&argv(&["--nope"])), 2);
    }
}
