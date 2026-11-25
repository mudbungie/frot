use crate::css::query_all;
use crate::dom::{Document, NodeKind};

fn ids(html: &str, selector: &str) -> String {
    let doc = Document::parse(html);
    let hits = query_all(&doc, Document::ROOT, selector).unwrap();
    let id = |&n: &u32| match &doc.node(n).kind {
        NodeKind::Element(e) => e.attr("id").unwrap_or("-").to_string(),
        other => panic!("matched a non-element: {other:?}"),
    };
    let named = hits.iter().map(id).filter(|i| i != "-");
    named.collect::<Vec<_>>().join(" ")
}

/// The corpus page, whose four lists are what Chrome 158 answered.
const PAGE: &str = include_str!("../../../tests/fixtures/fidelity/validity.html");

#[test]
fn the_corpus_page_reads_as_chrome_does() {
    assert_eq!(
        ids(PAGE, ":valid"),
        "t_ok t_ws f_good t_free b_sub b_bogus i_submit fs_empty fs_good i_fs2 ta_ok \
         n_req cb_on rb1 rb2 ra_solo ra_solo_on s_pick s_nonempty s_group s_multi_on s_free \
         file em_ok em_multi_ok em_ws url_ok p_ok p_empty p_alt p_broken p_num n_in n_blank \
         n_free rg rg_plain d_in d_bad tm_rev tx_range st_ok st_any st_val st_neg len minlen col"
    );
    assert_eq!(
        ids(PAGE, ":invalid"),
        "f_bad t_req fs_bad i_fs ta_req n_bad cb_req ra1 ra2 s_ph s_ph_text s_multi s_size \
         s_empty file_req em_bad em_multi em_dash url_bad p_bad n_over n_under n_rev d_under \
         mo_over wk_over tm_in tm_rev_out dt_over st_bad"
    );
    assert_eq!(
        ids(PAGE, ":in-range"),
        "n_bad n_in n_blank rg rg_plain d_in d_bad tm_in tm_rev st_bad st_ok st_any"
    );
    assert_eq!(
        ids(PAGE, ":out-of-range"),
        "n_over n_under n_rev d_under mo_over wk_over tm_rev_out dt_over"
    );
}

#[test]
fn dates_and_times_that_do_not_parse_are_empty() {
    // Each malformed value is sanitized to empty: in range, and missing when
    // required.
    let bad = [
        ("date", "2020"),
        ("date", "2020-01"),
        ("date", "20-01-01"),
        ("date", "0000-01-01"),
        ("date", "2020-13-01"),
        ("date", "2020-1-01"),
        ("date", "2020-a1-01"),
        ("date", "99999999999-01-01"),
        ("date", "2021-02-29"),
        ("date", "1900-02-29"),
        ("month", "2020"),
        ("month", "20-01"),
        ("month", "2020-13"),
        ("week", "2020-05"),
        ("week", "20-W05"),
        ("week", "2020-W54"),
        ("time", "1200"),
        ("time", "24:00"),
        ("time", "12:60"),
        ("time", "12:00:60"),
        ("time", "12:00:00.1234"),
        ("time", "12:00:00.a"),
        ("datetime-local", "2020-01-01"),
        ("datetime-local", "2020-13-01T10:00"),
        ("datetime-local", "2020-01-01T25:00"),
    ];
    for (kind, value) in bad {
        let html = format!("<input id=x type={kind} required min=0001-01 value='{value}'>");
        assert_eq!(ids(&html, ":invalid"), "x", "{kind} {value}");
        assert_eq!(ids(&html, ":in-range"), "x", "{kind} {value}");
    }
    // Leap days by the Gregorian rule, and every good shape.
    for value in ["2020-02-29", "2000-02-29", "2021-02-28"] {
        let html = format!("<input id=x type=date min=2000-01-01 value={value}>");
        assert_eq!(ids(&html, ":valid:in-range"), "x", "{value}");
    }
    let good = "<input id=a type=time value=12:00 min=11:00><input id=b type=time value=12:00:30 \
        min=11:00 step=30><input id=c type=datetime-local value='2020-01-01 10:00' min=2020-01-01T09:00>";
    assert_eq!(ids(good, ":valid:in-range"), "a b c");
}

#[test]
fn the_value_constraints_one_by_one() {
    let html = "<input id=e1 type=email value='a.b+c@d-e.f'><input id=e2 type=email value='@b'>\
        <input id=e3 type=email value='a(b@c'><input id=e4 type=email value='a@b..c'>\
        <input id=e5 type=email value='a@b_c'><input id=e6 type=email value='a@b-'>\
        <input id=p1 type=search pattern='x' value=y><input id=p2 type=EMAIL pattern='[a-z]@[a-z]' value='a@b'>\
        <input id=p3 type=email multiple pattern='[a-z]@[a-z]' value='a@b, cc@d'>\
        <input id=n1 type=number value=3 min=bogus><input id=n2 type=number value=2.5 step=0 min=0>\
        <input id=n3 type=number value=1.5 min=0><input id=n4 type=bogus min=1 max=2 value=5>\
        <button id=b1 type=number min=1 max=2 value=5></button><input id=r1 type=radio name=z><input id=d1 type=number min=1 value=9 disabled>";
    assert_eq!(ids(html, ":valid"), "e1 p2 n1 n4 b1 r1");
    assert_eq!(ids(html, ":invalid"), "e2 e3 e4 e5 e6 p1 p3 n2 n3");
    assert_eq!(ids(html, ":in-range"), "n2 n3");
    assert_eq!(ids(html, ":out-of-range"), "");
}
