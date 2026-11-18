//! The predicate: equality of two projected sequences, and — when they are not
//! equal — the deltas, in the terms the ledger declares them in.
//!
//! A delta is one line present on exactly one side: `- line` is Chrome's and
//! frot lacks it, `+ line` is frot's and Chrome lacks it. There is no distance,
//! score or threshold here (`fidelity.md` F2): two sequences are equal or they
//! are not, and the delta list is what a human dispositions.

/// The lines on which `chrome` and `frot` differ, in sequence order. Empty
/// exactly when the two are equal.
pub fn delta(chrome: &[String], frot: &[String]) -> Vec<String> {
    // Longest-common-subsequence table over the suffixes: `keep[i][j]` is the
    // most lines `chrome[i..]` and `frot[j..]` can share in order.
    let mut keep = vec![vec![0usize; frot.len() + 1]; chrome.len() + 1];
    for i in (0..chrome.len()).rev() {
        for j in (0..frot.len()).rev() {
            keep[i][j] = if chrome[i] == frot[j] {
                keep[i + 1][j + 1] + 1
            } else {
                keep[i + 1][j].max(keep[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < chrome.len() || j < frot.len() {
        if i < chrome.len() && j < frot.len() && chrome[i] == frot[j] {
            i += 1;
            j += 1;
        } else if j == frot.len() || (i < chrome.len() && keep[i + 1][j] >= keep[i][j + 1]) {
            out.push(format!("- {}", chrome[i]));
            i += 1;
        } else {
            out.push(format!("+ {}", frot[j]));
            j += 1;
        }
    }
    out
}

/// F4, total disposition, in both directions: every actual delta must be
/// declared, and every declared delta must still appear. Returns the complaint
/// lines; empty means the channel holds.
pub fn undisposed(actual: &[String], declared: &[&str]) -> Vec<String> {
    let mut left: Vec<&str> = declared.to_vec();
    let mut out = Vec::new();
    for d in actual {
        match left.iter().position(|l| l == d) {
            Some(at) => drop(left.remove(at)),
            None => out.push(format!("undeclared delta: {d}")),
        }
    }
    out.extend(
        left.iter().map(|l| {
            format!("declared delta no longer appears (fixed? delete its ledger row): {l}")
        }),
    );
    out
}

fn lines(s: &[&str]) -> Vec<String> {
    s.iter().map(|l| l.to_string()).collect()
}

#[test]
fn equal_sequences_have_no_delta() {
    assert!(delta(&lines(&["a", "b"]), &lines(&["a", "b"])).is_empty());
    assert!(delta(&[], &[]).is_empty());
}

#[test]
fn a_delta_names_the_side_that_has_the_line() {
    let d = delta(&lines(&["a", "b", "c"]), &lines(&["a", "x", "c", "d"]));
    assert_eq!(d, ["- b", "+ x", "+ d"]);
    assert_eq!(
        delta(&lines(&["first", "second"]), &lines(&["firstsecond"])).len(),
        3
    );
}

#[test]
fn an_undeclared_delta_and_a_vanished_one_both_fail() {
    let actual = lines(&["+ x", "+ x"]);
    assert!(undisposed(&actual, &["+ x", "+ x"]).is_empty());
    assert_eq!(
        undisposed(&actual, &["+ x"]).len(),
        1,
        "a second copy is undeclared"
    );
    assert_eq!(
        undisposed(&[], &["+ x"]).len(),
        1,
        "a residual may not vanish"
    );
    assert_eq!(undisposed(&actual, &[]).len(), 2);
}
