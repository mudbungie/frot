//! The delivery seam (posix.md §4.4): a failed stdout write must not exit as
//! success. The non-pipe failures are unit-tested here through the `dyn Write`
//! signature; the true SIGPIPE death (141) and the real EBADF/ENOSPC devices
//! are asserted at the process boundary by `scripts/posix-suite.sh`.

use super::*;

/// A sink whose every write fails with the given raw OS error.
struct FailWrite(i32);

impl Write for FailWrite {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::from_raw_os_error(self.0))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

#[test]
fn write_failure_writes_diagnostic_and_exits_3() {
    let mut err = Vec::new();
    let code = run_io(
        &argv(&["--version"]),
        &mut FailWrite(libc::ENOSPC),
        &mut err,
    );
    assert_eq!(code, 3);
    let msg = String::from_utf8(err).unwrap();
    assert!(msg.contains("frot: stdout write failed"), "{msg}");
}

#[test]
fn stderr_gone_too_still_exits_3_without_panic() {
    let code = run_io(
        &argv(&["--version"]),
        &mut FailWrite(libc::ENOSPC),
        &mut FailWrite(libc::EBADF),
    );
    assert_eq!(code, 3);
}

#[test]
fn envelope_write_failure_exits_3_not_by_envelope_status() {
    // An error envelope would exit 1 if delivered; undelivered, it exits 3.
    let mut err = Vec::new();
    let code = run_io(
        &argv(&["not a url", "--out", "text"]),
        &mut FailWrite(libc::EBADF),
        &mut err,
    );
    assert_eq!(code, 3);
    assert!(String::from_utf8(err)
        .unwrap()
        .contains("stdout write failed"));
}

#[test]
fn broken_pipe_raises_sigpipe_through_the_injected_disposition() {
    // SIG_IGN keeps the raised SIGPIPE from terminating the test process
    // (the Rust runtime already holds that disposition, so this re-set is a
    // no-op process-wide); production injects SIG_DFL through `run_io`, and
    // the suite asserts the resulting 141 at the process boundary.
    let mut err = Vec::new();
    let code = run_with(
        &argv(&["--version"]),
        &mut FailWrite(libc::EPIPE),
        &mut err,
        libc::SIG_IGN,
    );
    assert_eq!(code, 3);
    // The pipe path is silent: the reader is gone and death is the message.
    assert!(err.is_empty());
}
