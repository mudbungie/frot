use super::*;

fn tmp_path(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("frot-fetch-file-{}-{}", std::process::id(), name));
    p
}

fn file_url(p: &std::path::Path) -> String {
    url::Url::from_file_path(p).unwrap().to_string()
}

#[test]
fn reads_a_local_html_file() {
    let p = tmp_path("ok.html");
    std::fs::write(&p, "<title>Local</title><p>hi</p>").unwrap();
    let r = fetch(&file_url(&p), &[]).unwrap();
    assert_eq!(r.status, None);
    assert!(r.headers.is_empty());
    assert_eq!(r.charset, "utf-8");
    assert!(r.body.contains("hi"));
    assert!(r.final_url.starts_with("file://"));
    std::fs::remove_file(&p).unwrap();
}

#[test]
fn meta_charset_is_sniffed_from_file_bytes() {
    let p = tmp_path("latin.html");
    let mut bytes = b"<meta charset=\"windows-1252\"><p>caf".to_vec();
    bytes.push(0xE9); // 'é' in windows-1252; invalid as UTF-8
    bytes.extend_from_slice(b"</p>");
    std::fs::write(&p, &bytes).unwrap();
    let r = fetch(&file_url(&p), &[]).unwrap();
    assert_eq!(r.charset, "windows-1252");
    assert!(r.body.contains("café"));
    std::fs::remove_file(&p).unwrap();
}

#[test]
fn missing_file_is_fetch_file_error() {
    let p = tmp_path("does-not-exist.html");
    let e = fetch(&file_url(&p), &[]).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_FILE);
}

#[test]
fn directory_is_fetch_file_error() {
    let p = tmp_path("a-dir");
    std::fs::create_dir_all(&p).unwrap();
    let e = fetch(&file_url(&p), &[]).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_FILE);
    std::fs::remove_dir(&p).unwrap();
}

#[test]
fn file_url_with_remote_host_is_fetch_url_error() {
    let e = fetch("file://remotehost/etc/hosts", &[]).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_URL);
    assert!(e.message.contains("not a local file path"));
}

/// The size gate is wired into the `file://` read, not just available beside
/// it: an over-limit file is refused before its bytes are ever read. The file
/// is sparse (`set_len`), so this costs no disk.
#[test]
fn oversized_file_is_refused_by_the_body_gate() {
    let p = tmp_path("huge.html");
    let f = std::fs::File::create(&p).unwrap();
    f.set_len(MAX_BODY_BYTES + 1).unwrap();
    drop(f);
    let e = fetch(&file_url(&p), &[]).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_BODY);
    assert!(e.message.contains("limit"), "message was {:?}", e.message);
    std::fs::remove_file(&p).unwrap();
}

#[test]
fn body_len_gate_accepts_at_limit_and_rejects_over() {
    assert!(check_body_len(MAX_BODY_BYTES).is_ok());
    let e = check_body_len(MAX_BODY_BYTES + 1).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_BODY);
    assert!(e.message.contains("limit"));
}
