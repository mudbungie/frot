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

// The minimized DOM-move page (bl-ae88), served to a REAL child process: the
// bug it pins is an abort, and an abort cannot be observed from inside the
// process it kills — a panic in a syscall closure is a panic across an `extern
// "C"` frame, so it never unwinds into a test failure. Only the exit status and
// the single envelope on stdout prove the contract held.
const MOVE_PAGE: &str = include_str!("fixtures/js/dom-move.html");

#[test]
fn same_parent_dom_moves_survive_the_process_boundary() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body(MOVE_PAGE)
        .create();
    let url = server.url();
    let out = bin()
        .args([&url, "--js", "--js-errors", "--out", "text"])
        .output()
        .expect("spawn frot");
    // 134 is the abort this fixture used to take: SIGABRT, no envelope at all.
    assert_eq!(out.status.code(), Some(0), "{:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.lines().count(), 1, "exactly one envelope: {stdout}");
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("envelope JSON");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["js"]["errors"], 0);
    // Child order after each move, and the mutation records the same seam
    // delivered — both verbatim from Chrome on this fixture, except the final
    // `stale-ref` step (Chrome throws NotFoundError there; the fixture's comment
    // records why frot appends) and the two records that step then adds.
    assert_eq!(
        v["out"],
        "DOM move\n\
         append-last=abc/d move-back=bca/d move-fwd=abc/d self-mid=abc/d \
         self-last=abc/d cross=ac/db stale-ref=ca/d\n\
         p-c p+c p-a p+a p-a p+a p-b p+b p-c p+c p-b q+b q-b p-a p+a"
    );
}

#[test]
fn readme_tagline_is_the_crate_description_verbatim() {
    // Cargo.toml `description` is the tagline's single authoritative home
    // (it is the crates.io headline); README.md repeats it verbatim rather
    // than restating it independently. This pin is what keeps them from
    // drifting apart.
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
        .expect("read README.md");
    assert!(readme.contains(env!("CARGO_PKG_DESCRIPTION")));
}
