//! The AX-tree shapes behind the `bl-a189` field report, each reduced to the
//! markup that produced the delta and asserted against the Chrome full-AX
//! counts recorded there.

use super::*;

fn tree(html: &str) -> Value {
    ax_tree(&Document::parse(html), None, None)
}

fn count_role(v: &Value, role: &str) -> usize {
    let mut n = 0;
    if let Value::Array(arr) = v {
        for item in arr {
            if item["role"] == role {
                n += 1;
            }
            n += count_role(&item["children"], role);
        }
    }
    n
}

/// Wikipedia "Accessibility": 14 `th scope=row` plus 4 column headers.
/// Chrome: 14 rowheader, 4 columnheader. frot used to say 0 and 18.
#[test]
fn wikipedia_row_headers() {
    let mut html = String::from("<table><tr><th>A</th><th>B</th><th>C</th><th>D</th></tr>");
    for i in 0..14 {
        html.push_str(&format!(
            "<tr><th scope=row>R{i}</th><td>x</td><td>y</td><td>z</td></tr>"
        ));
    }
    html.push_str("</table>");
    let v = tree(&html);
    assert_eq!(count_role(&v, "rowheader"), 14);
    assert_eq!(count_role(&v, "columnheader"), 4);
}

/// USA.gov: one hidden affiliate input plus one search input. Chrome exposes
/// only the searchbox; frot used to add an unnamed textbox.
#[test]
fn usa_gov_hidden_input_is_not_exposed() {
    let v = tree("<form aria-label=Search><input type=hidden name=affiliate value=usagov><input type=search aria-label='Search USA.gov'></form>");
    assert_eq!(count_role(&v, "textbox"), 0);
    assert_eq!(count_role(&v, "searchbox"), 1);
}

/// USA.gov: 7 `<section>`s, 4 unnamed. Chrome exposes 3 regions.
#[test]
fn usa_gov_unnamed_sections_are_not_regions() {
    let mut html = String::new();
    for i in 0..3 {
        html.push_str(&format!("<section aria-label='S{i}'>x</section>"));
    }
    for _ in 0..4 {
        html.push_str("<section>x</section>");
    }
    assert_eq!(count_role(&tree(&html), "region"), 3);
}

/// Python docs: two search forms inside two `<search>` elements. Chrome: 2
/// search landmarks, 0 nested form landmarks.
#[test]
fn python_docs_search_landmarks() {
    let html = "<search><form><input type=search></form></search>\
                <search><form><input type=search></form></search>";
    let v = tree(html);
    assert_eq!(count_role(&v, "search"), 2);
    assert_eq!(count_role(&v, "form"), 0);
}

/// Stanford: one page header plus 10 section headers, one page footer plus a
/// blockquote footer. Chrome: banner 1, sectionheader 10, contentinfo 1,
/// sectionfooter 1. frot used to say banner 11, contentinfo 2.
#[test]
fn stanford_header_and_footer_scoping() {
    let mut html = String::from("<header>page</header>");
    for i in 0..10 {
        html.push_str(&format!(
            "<section aria-label='S{i}'><header>h{i}</header></section>"
        ));
    }
    html.push_str("<blockquote><footer>quoted</footer></blockquote><footer>page</footer>");
    let v = tree(&html);
    assert_eq!(count_role(&v, "banner"), 1);
    assert_eq!(count_role(&v, "sectionheader"), 10);
    assert_eq!(count_role(&v, "contentinfo"), 1);
    assert_eq!(count_role(&v, "sectionfooter"), 1);
}

/// W3C Forms Validation: `input type=number` is a spinbutton, not a textbox.
#[test]
fn w3c_number_input_is_a_spinbutton() {
    let v = tree("<label for=n>Count</label><input id=n type=number>");
    assert_eq!(count_role(&v, "spinbutton"), 1);
    assert_eq!(count_role(&v, "textbox"), 0);
}
