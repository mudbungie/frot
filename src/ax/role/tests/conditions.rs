//! The conditional HTML-AAM mappings, each with its inverse.
//!
//! Every expectation here is a Chrome full-AX observation from the report in
//! `bl-a189` or the condition the HTML-AAM states for it.

use super::*;

/// Every `input` type, with the role Chrome exposes for it. `hidden` is the
/// only one that is not exposed at all.
const INPUT_TYPES: &[(&str, Option<&str>)] = &[
    ("hidden", None),
    ("button", Some("button")),
    ("image", Some("button")),
    ("reset", Some("button")),
    ("submit", Some("button")),
    ("checkbox", Some("checkbox")),
    ("radio", Some("radio")),
    ("range", Some("slider")),
    ("number", Some("spinbutton")),
    ("search", Some("searchbox")),
    ("text", Some("textbox")),
    ("email", Some("textbox")),
    ("tel", Some("textbox")),
    ("url", Some("textbox")),
    ("password", Some("textbox")),
    ("color", Some("textbox")),
    ("file", Some("textbox")),
    ("date", Some("textbox")),
    ("datetime-local", Some("textbox")),
    ("month", Some("textbox")),
    ("time", Some("textbox")),
    ("week", Some("textbox")),
    ("wibble", Some("textbox")),
];

#[test]
fn every_input_type_maps() {
    for (ty, want) in INPUT_TYPES {
        assert_eq!(r(&format!("<input id=t type='{ty}'>")), *want, "type={ty}");
    }
}

#[test]
fn input_type_is_ascii_case_insensitive() {
    assert_eq!(r("<input id=t type=CHECKBOX>"), Some("checkbox"));
    assert_eq!(r("<input id=t type=Hidden>"), None);
}

#[test]
fn input_without_type_is_a_textbox() {
    assert_eq!(r("<input id=t>"), Some("textbox"));
}

#[test]
fn a_list_attribute_makes_text_entry_a_combobox() {
    for ty in ["text", "email", "tel", "url", "search"] {
        let html = format!("<input id=t type='{ty}' list=d><datalist id=d></datalist>");
        assert_eq!(r(&html), Some("combobox"), "type={ty}");
    }
    // Non-text-entry types ignore `list` entirely.
    assert_eq!(r("<input id=t type=number list=d>"), Some("spinbutton"));
    assert_eq!(r("<input id=t type=checkbox list=d>"), Some("checkbox"));
}

#[test]
fn th_scope_decides_the_header_axis() {
    for (scope, want) in [
        ("row", "rowheader"),
        ("rowgroup", "rowheader"),
        ("ROW", "rowheader"),
        ("col", "columnheader"),
        ("colgroup", "columnheader"),
    ] {
        let html = format!("<table><tr><th id=t scope='{scope}'>H</th><th>H2</th></tr></table>");
        assert_eq!(r(&html), Some(want), "scope={scope}");
    }
}

#[test]
fn a_scopeless_th_opening_a_data_row_is_a_rowheader() {
    // Wikipedia's accessibility tables: the first cell of each body row.
    let html = "<table><tr><!--c--><th id=t>Name</th><td>Value</td></tr></table>";
    assert_eq!(r(html), Some("rowheader"));
}

#[test]
fn a_scopeless_th_in_an_all_header_row_is_a_columnheader() {
    let html = "<table><tr><th id=t>A</th><th>B</th></tr></table>";
    assert_eq!(r(html), Some("columnheader"));
}

#[test]
fn a_scopeless_th_that_does_not_open_its_row_is_a_columnheader() {
    let html = "<table><tr><td>V</td><th id=t>A</th></tr></table>";
    assert_eq!(r(html), Some("columnheader"));
}

#[test]
fn cells_follow_the_ancestor_tables_role() {
    assert_eq!(r("<table><tr><td id=t>d</td></tr></table>"), Some("cell"));
    for grid in ["grid", "treegrid"] {
        let html = format!("<table role='{grid}'><tr><td id=t>d</td></tr></table>");
        assert_eq!(r(&html), Some("gridcell"), "{grid}");
    }
    // Headers keep their axis roles inside a grid.
    let html = "<table role=grid><tr><th id=t scope=row>H</th><td>d</td></tr></table>";
    assert_eq!(r(html), Some("rowheader"));
}

#[test]
fn a_section_is_a_region_only_when_named() {
    assert_eq!(r("<section id=t>x</section>"), Some("generic"));
    assert_eq!(
        r("<section id=t aria-label='Intro'>x</section>"),
        Some("region")
    );
    assert_eq!(r("<section id=t title='Intro'>x</section>"), Some("region"));
    assert_eq!(
        r("<h2 id=h>Intro</h2><section id=t aria-labelledby=h>x</section>"),
        Some("region")
    );
    // A name that computes to nothing is no name.
    assert_eq!(
        r("<section id=t aria-label='  '>x</section>"),
        Some("generic")
    );
    assert_eq!(
        r("<section id=t aria-labelledby=missing>x</section>"),
        Some("generic")
    );
}

#[test]
fn a_form_is_a_landmark_only_when_named() {
    // python.org: a search form inside <search> is not a second landmark.
    assert_eq!(r("<form id=t><input></form>"), Some("generic"));
    assert_eq!(
        r("<form id=t aria-label='Site search'><input></form>"),
        Some("form")
    );
}

#[test]
fn an_aside_is_complementary_at_top_level_and_named_inside_a_section() {
    assert_eq!(r("<aside id=t>x</aside>"), Some("complementary"));
    assert_eq!(
        r("<article><aside id=t>x</aside></article>"),
        Some("generic")
    );
    assert_eq!(
        r("<article><aside id=t aria-label='Notes'>x</aside></article>"),
        Some("complementary")
    );
}

#[test]
fn header_and_footer_are_page_landmarks_only_outside_a_sectioning_ancestor() {
    assert_eq!(r("<header id=t>x</header>"), Some("banner"));
    assert_eq!(r("<footer id=t>x</footer>"), Some("contentinfo"));
    assert_eq!(r("<div><header id=t>x</header></div>"), Some("banner"));
}

#[test]
fn a_sectioned_header_or_footer_is_section_scoped() {
    for tag in ["article", "aside", "main", "nav", "section"] {
        let html = format!("<{tag}><header id=t>x</header></{tag}>");
        assert_eq!(r(&html), Some("sectionheader"), "{tag}");
    }
    // Sectioning *roots* scope them too — Stanford's blockquote footer.
    for html in [
        "<blockquote><footer id=t>x</footer></blockquote>",
        "<details><footer id=t>x</footer></details>",
        "<dialog><footer id=t>x</footer></dialog>",
        "<figure><footer id=t>x</footer></figure>",
        "<fieldset><footer id=t>x</footer></fieldset>",
        "<table><tr><td><footer id=t>x</footer></td></tr></table>",
    ] {
        assert_eq!(r(html), Some("sectionfooter"), "{html}");
    }
}

#[test]
fn a_list_item_needs_a_list() {
    for tag in ["ul", "ol", "menu"] {
        let html = format!("<{tag}><li id=t>x</li></{tag}>");
        assert_eq!(r(&html), Some("listitem"), "{tag}");
    }
    assert_eq!(r("<div><li id=t>x</li></div>"), Some("generic"));
}

#[test]
fn empty_alt_is_decorative_unless_something_else_names_it() {
    assert_eq!(r("<img id=t alt=''>"), Some("presentation"));
    assert_eq!(r("<img id=t alt='Logo'>"), Some("img"));
    assert_eq!(r("<img id=t>"), Some("img"));
    assert_eq!(r("<img id=t alt='' aria-label='Logo'>"), Some("img"));
    assert_eq!(r("<img id=t alt='' title='Logo'>"), Some("img"));
}
