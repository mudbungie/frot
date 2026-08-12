//! No personal network identity in the tree (`bl-f521`).
//!
//! A field trial's egress address is the author's home line: permanent once
//! published, and worth no reproducibility to anyone else. These gates read
//! the class — any globally-routable IPv4 literal, any ASN — so the scrubbed
//! address never has to be named to be kept out.

use super::repo;
use std::fs;
use std::process::Command;

/// The tracked text of the repo — what publishing it would hand out — minus
/// the two trees where arbitrary digits are expected: `tests/fixtures/`
/// (vendored third-party bundles) and `Cargo.lock` (resolver output).
///
/// Enumerated with `git ls-files` rather than a directory walk, so untracked
/// scratch files and build output are out of scope by construction.
fn tracked_text() -> Vec<(String, String)> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo(""))
        .output()
        .expect("run git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8(out.stdout)
        .expect("git ls-files paths are utf8")
        .split('\0')
        .filter(|p| !p.is_empty() && !p.starts_with("tests/fixtures/") && *p != "Cargo.lock")
        .filter_map(|p| fs::read_to_string(repo(p)).ok().map(|t| (p.to_string(), t)))
        .collect()
}

/// Every dotted quad in `text`, as `(1-based line, octets)`.
///
/// A candidate is a maximal run of digits and dots that is not glued to a
/// letter or digit on either side, so `v1.2.3.4` and `jquery-3.7.1.min.js`
/// are versions and filenames, not addresses.
fn dotted_quads(text: &str) -> Vec<(usize, [u8; 4])> {
    let mut found = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !bytes[i].is_ascii_digit() {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let glued_left = start > 0 && bytes[start - 1].is_ascii_alphanumeric();
            let glued_right = i < bytes.len() && bytes[i].is_ascii_alphanumeric();
            if !glued_left && !glued_right {
                if let Some(quad) = as_quad(line[start..i].trim_matches('.')) {
                    found.push((n + 1, quad));
                }
            }
        }
    }
    found
}

/// Exactly four dot-separated octets, each in range, or nothing.
fn as_quad(run: &str) -> Option<[u8; 4]> {
    let mut parts = run.split('.');
    let mut quad = [0u8; 4];
    for octet in quad.iter_mut() {
        *octet = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(quad)
}

/// Whether `quad` is outside globally-routable space — the addresses a doc or
/// a test may legitimately name (RFC 1918 private, 5735 special-purpose, 5737
/// documentation, 6598 CGNAT, plus multicast and reserved).
fn is_reserved(quad: [u8; 4]) -> bool {
    match quad {
        [0, ..] | [10, ..] | [127, ..] | [169, 254, ..] | [192, 168, ..] => true,
        [100, b, ..] => (64..=127).contains(&b),
        [172, b, ..] => (16..=31).contains(&b),
        [192, 0, 0 | 2, _] | [192, 88, 99, _] => true,
        [198, 18 | 19, ..] | [198, 51, 100, _] | [203, 0, 113, _] => true,
        [a, ..] => a >= 224,
    }
}

/// Offsets of `AS<digits>` mentions — an autonomous-system number names the
/// ISP behind an address about as precisely as the address does.
fn asn_mentions(text: &str) -> Vec<usize> {
    let mut found = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        for i in 0..bytes.len().saturating_sub(2) {
            let starts = (i == 0 || !bytes[i - 1].is_ascii_alphanumeric())
                && bytes[i] == b'A'
                && bytes[i + 1] == b'S';
            let digits = bytes[i + 2..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count();
            if starts && digits >= 2 {
                found.push(n + 1);
            }
        }
    }
    found
}

#[test]
fn no_public_ip_literal_in_the_tree() {
    let leaks: Vec<String> = tracked_text()
        .iter()
        .flat_map(|(path, text)| {
            dotted_quads(text)
                .into_iter()
                .filter(|(_, quad)| !is_reserved(*quad))
                .map(move |(line, _)| format!("{path}:{line}"))
        })
        .collect();
    assert!(
        leaks.is_empty(),
        "public IPv4 literal(s) in the tree: {leaks:?}\n\
         A measurement's egress address is personal network identity, it is \
         permanent once published, and it buys no reproducibility. Cite 'the \
         reference egress' as docs/design/identity.md §3 does; if a doc needs \
         a literal, use an RFC 5737 one (192.0.2.x/198.51.100.x/203.0.113.x)."
    );
}

#[test]
fn no_asn_reference_in_the_tree() {
    let leaks: Vec<String> = tracked_text()
        .iter()
        .flat_map(|(path, text)| {
            asn_mentions(text)
                .into_iter()
                .map(move |line| format!("{path}:{line}"))
        })
        .collect();
    assert!(
        leaks.is_empty(),
        "autonomous-system number(s) in the tree: {leaks:?}\n\
         An ASN names the ISP behind a measurement as surely as the address \
         does — the scrubbed leak was an address *and* its 'AS…, city' gloss."
    );
}

#[test]
fn quad_scanner_reads_addresses_and_ignores_versions() {
    let hits = |s: &str| {
        dotted_quads(s)
            .into_iter()
            .map(|(_, q)| q)
            .collect::<Vec<_>>()
    };
    assert_eq!(hits("bind `203.0.113.7:80` now"), vec![[203, 0, 113, 7]]);
    assert_eq!(
        hits("the address is 198.51.100.9."),
        vec![[198, 51, 100, 9]]
    );
    // Not addresses: a version behind a letter, a filename glued to one, too
    // few or too many groups, and an octet out of range.
    for text in [
        "v1.2.3.4 shipped",
        "jquery-3.7.1.min.js",
        "1.2.3 and 1.2.3.4.5",
        "256.1.1.1",
        "no digits here",
    ] {
        assert!(hits(text).is_empty(), "{text} must not read as an address");
    }
}

#[test]
fn reserved_ranges_cover_every_non_routable_block() {
    for quad in [
        [0, 0, 0, 0],
        [10, 1, 2, 3],
        [127, 0, 0, 1],
        [169, 254, 1, 1],
        [192, 168, 1, 1],
        [100, 64, 0, 1],
        [172, 16, 0, 1],
        [192, 0, 0, 8],
        [192, 0, 2, 1],
        [192, 88, 99, 1],
        [198, 18, 0, 1],
        [198, 51, 100, 1],
        [203, 0, 113, 1],
        [239, 0, 0, 1],
        [255, 255, 255, 255],
    ] {
        assert!(is_reserved(quad), "{quad:?} is not globally routable");
    }
    for quad in [
        [8, 8, 8, 8],
        [100, 128, 0, 1],
        [172, 32, 0, 1],
        [198, 20, 0, 1],
    ] {
        assert!(!is_reserved(quad), "{quad:?} is a public address");
    }
}

#[test]
fn asn_scanner_reads_numbers_and_ignores_words() {
    assert_eq!(asn_mentions("measured from [redacted-egress-asn] today"), vec![1]);
    for text in ["ASCII text", "the AS clause", "BASE12 identifier", "AS1"] {
        assert!(
            asn_mentions(text).is_empty(),
            "{text} must not read as an ASN"
        );
    }
}
