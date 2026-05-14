//! Integration tests that spawn the compiled `frot` binary.
//! Required so cargo-llvm-cov picks up `main()` coverage.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_frot"))
}

#[test]
fn no_args_exits_2() {
    let out = bin().output().expect("spawn frot");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no arguments"));
}

#[test]
fn help_exits_0() {
    let out = bin().arg("--help").output().expect("spawn frot");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("usage: frot"));
}

#[test]
fn valid_args_stub_exits_2() {
    let out = bin()
        .args(["https://example.com/", "--out", "text"])
        .output()
        .expect("spawn frot");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("phase 0"));
}
