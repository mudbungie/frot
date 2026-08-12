//! The delivery seam: argv → one envelope → stdout, and the exit status that
//! reports what happened to it.
//!
//! Exit codes (derived from the authoritative table, `docs/design/posix.md`
//! §4.4; gated by `scripts/posix-suite.sh`):
//! - `0` — help/version, or `ok`/`needs` envelope emitted.
//! - `1` — `error` envelope emitted (fetch failure or unsupported view).
//! - `2` — usage error (no envelope emitted; message on stderr).
//! - `3` — the product could not be written to stdout (envelope lost;
//!   best-effort diagnostic on stderr).
//!
//! A broken pipe is not status `3`: frot restores SIGPIPE's default
//! disposition at the stdout seam and re-raises, dying with the true 141
//! status pipelines expect (posix.md §5.2).

use std::io::Write;

use super::build_envelope;
use crate::cli;
use crate::envelope::StatusKind;
use crate::js::Bounds;

pub fn run(argv: &[String]) -> u8 {
    run_io(argv, &mut std::io::stdout(), &mut std::io::stderr())
}

pub(crate) fn run_io(argv: &[String], out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    run_bounded(argv, out, err, Bounds::shipping())
}

/// [`run_io`] with the run's two §5 JS budgets taken through the call
/// signature, exactly as `pipe_disposition` is below: production supplies the
/// host clocks ([`Bounds::shipping`]), and the golden fixture suite — which
/// serves every byte from an in-process mock and so has no real network wait
/// to meter — supplies a frozen network window it advances itself (bl-c81a).
pub(crate) fn run_bounded(
    argv: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
    bounds: Bounds,
) -> u8 {
    run_with(argv, out, err, libc::SIG_DFL, bounds)
}

/// The pipeline with the broken-pipe SIGPIPE disposition taken through the
/// call signature: production (`run_io`) passes `SIG_DFL`, so a broken-pipe
/// delivery dies with the true 141 status; tests pass `SIG_IGN` to walk the
/// same path and survive.
fn run_with(
    argv: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
    pipe_disposition: libc::sighandler_t,
    bounds: Bounds,
) -> u8 {
    let (product, exit) = match cli::parse(argv) {
        Ok(args) => {
            let env = build_envelope(&args, bounds);
            let exit = match env.status {
                StatusKind::Ok | StatusKind::Needs => 0,
                StatusKind::Error => 1,
            };
            (env.to_json_string(), exit)
        }
        Err(e) if e.is_help_or_version() => (e.to_string(), 0),
        Err(e) => {
            let _ = writeln!(err, "{}", e);
            return 2;
        }
    };
    // The one seam where the product is delivered (posix.md §4.2: stdout
    // carries exactly one product). A failed write here means the product is
    // lost, and the exit status must say so (§4.4).
    match writeln!(out, "{product}") {
        Ok(()) => exit,
        // Reader gone (EPIPE). Rust ignores SIGPIPE process-wide so socket
        // writes surface EPIPE as ordinary fetch errors; only here, at the
        // stdout seam, the pipeline convention applies: restore the default
        // disposition and re-raise so the shell observes 141 (§5.2).
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
            unsafe {
                libc::signal(libc::SIGPIPE, pipe_disposition);
                libc::raise(libc::SIGPIPE);
            }
            // Under SIG_DFL the raise terminated above; reached only when the
            // injected disposition keeps SIGPIPE ignored. The product is
            // still lost, so the lost-product status holds.
            3
        }
        // Any other write failure (ENOSPC, EBADF, …): the product is lost.
        // The diagnostic is best-effort — stderr may be gone too, and a
        // failed diagnostic must not recurse or panic.
        Err(e) => {
            let _ = writeln!(err, "frot: stdout write failed: {e}");
            3
        }
    }
}

#[cfg(test)]
mod tests;
