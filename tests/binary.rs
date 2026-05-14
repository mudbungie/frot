//! Integration tests that spawn the compiled `frot` binary.
//!
//! End-to-end runs against an in-process `mockito` server confirm that the
//! library's `run()` path is correctly invoked by `main` and that exit codes
//! match the documented envelope/usage contract.

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
fn invalid_url_emits_error_envelope_and_exits_1() {
    let out = bin()
        .args(["not a url", "--out", "text"])
        .output()
        .expect("spawn frot");
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("envelope JSON");
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["kind"], "fetch.url");
}

#[test]
fn fetch_against_local_mockito_server_emits_ok_envelope() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body("<p>spawned</p>")
        .create();
    let url = server.url();
    let out = bin()
        .args([&url, "--out", "text"])
        .output()
        .expect("spawn frot");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("envelope JSON");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "spawned");
}
